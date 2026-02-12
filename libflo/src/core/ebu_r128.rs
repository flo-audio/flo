use serde::{Deserialize, Serialize};

pub type FloSample = f32;

/// EBU R128 loudness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessMetrics {
    /// Integrated loudness in LUFS (LKFS)
    pub integrated_lufs: f64,
    /// Loudness range in LU (LRA)
    pub loudness_range_lu: f64,
    /// True peak in dBTP (oversampled)
    pub true_peak_dbtp: f64,
    /// Sample peak in dBFS
    pub sample_peak_dbfs: f64,
}

#[derive(Clone)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn new(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// K‑weighting per BS.1770: high‑shelf + high‑pass, per channel.
struct KWeighting {
    shelf: Vec<Biquad>,
    hp: Vec<Biquad>,
}

impl KWeighting {
    fn new(sample_rate: f64, channels: u8) -> Self {
        // From libebur128 / BS.1770
        // High‑shelf
        let f0 = 1681.974450955533;
        let g_db = 3.999843853973347;
        let q = 0.7071752369554196;

        let k = (std::f64::consts::PI * f0 / sample_rate).tan();
        let vh = 10.0_f64.powf(g_db / 20.0);
        let vb = vh.powf(0.4996667741545416);

        let mut pb = [0.0; 3];
        let mut pa = [0.0; 3];

        let a0 = 1.0 + k / q + k * k;
        pb[0] = (vh + vb * k / q + k * k) / a0;
        pb[1] = 2.0 * (k * k - vh) / a0;
        pb[2] = (vh - vb * k / q + k * k) / a0;
        pa[0] = 1.0;
        pa[1] = 2.0 * (k * k - 1.0) / a0;
        pa[2] = (1.0 - k / q + k * k) / a0;

        let shelf_proto = Biquad::new(pb[0], pb[1], pb[2], pa[1], pa[2]);

        // High‑pass
        let f0_hp = 38.13547087602444;
        let q_hp = 0.5003270373238773;
        let k_hp = (std::f64::consts::PI * f0_hp / sample_rate).tan();

        let a0_hp = 1.0 + k_hp / q_hp + k_hp * k_hp;
        let a1_hp = 2.0 * (k_hp * k_hp - 1.0) / a0_hp;
        let a2_hp = (1.0 - k_hp / q_hp + k_hp * k_hp) / a0_hp;

        // Numerator [1, -2, 1] as in libebur128’s combined form
        let hp_proto = Biquad::new(1.0, -2.0, 1.0, a1_hp, a2_hp);

        let mut shelf = Vec::with_capacity(channels as usize);
        let mut hp = Vec::with_capacity(channels as usize);
        for _ in 0..channels {
            shelf.push(shelf_proto.clone());
            hp.push(hp_proto.clone());
        }

        Self { shelf, hp }
    }

    #[inline]
    fn process(&mut self, x: f64, ch: usize) -> f64 {
        let y1 = self.shelf[ch].process(x);
        self.hp[ch].process(y1)
    }
}

/// Windowed‑sinc FIR oversampling for true peak (4×).
fn compute_true_peak(samples: &[FloSample], channels: u8, sample_rate: u32) -> f64 {
    if samples.is_empty() || channels == 0 {
        return -150.0;
    }

    let factor = 4u32;
    let oversample_rate = sample_rate as f64 * factor as f64;
    let cutoff = sample_rate as f64 * 0.45;
    let taps = 49usize;

    let mut coeffs = Vec::with_capacity(taps);
    let center = (taps - 1) as f64 / 2.0;

    for i in 0..taps {
        let n = i as f64 - center;
        let sinc = if n.abs() < 1e-12 {
            2.0 * cutoff / oversample_rate
        } else {
            (2.0 * cutoff * n / oversample_rate).sin() / (std::f64::consts::PI * n)
        };
        let window =
            0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (taps - 1) as f64).cos());
        coeffs.push(sinc * window);
    }

    let sum: f64 = coeffs.iter().sum();
    for c in &mut coeffs {
        *c /= sum;
    }

    let mut max_peak = 0.0f64;

    for ch in 0..channels as usize {
        let channel_samples: Vec<f64> = samples
            .iter()
            .skip(ch)
            .step_by(channels as usize)
            .map(|&s| s as f64)
            .collect();

        let len = channel_samples.len();
        if len == 0 {
            continue;
        }

        for i in 0..len {
            for sub in 0..factor {
                let pos = i as f64 + sub as f64 / factor as f64;
                let mut acc = 0.0;

                for (k, &h) in coeffs.iter().enumerate() {
                    let src = pos - center + k as f64;
                    if src >= 0.0 && src < len as f64 {
                        acc += channel_samples[src as usize] * h;
                    }
                }

                max_peak = max_peak.max(acc.abs());
            }
        }
    }

    if max_peak > 1e-9 {
        20.0 * max_peak.log10()
    } else {
        -150.0
    }
}

/// Compute EBU R128 loudness metrics from interleaved samples.
pub fn compute_ebu_r128_loudness(
    samples: &[FloSample],
    channels: u8,
    sample_rate: u32,
) -> LoudnessMetrics {
    if samples.is_empty() || channels == 0 {
        return LoudnessMetrics {
            integrated_lufs: -23.0,
            loudness_range_lu: 0.0,
            true_peak_dbtp: -150.0,
            sample_peak_dbfs: -150.0,
        };
    }

    let sr = sample_rate as f64;
    let hop_100ms = (sr * 0.1).round() as usize; // 100 ms hop
    let block_400ms = hop_100ms * 4; // 400 ms window

    // De‑interleave
    let frames = samples.len() / channels as usize;
    let mut per_channel: Vec<Vec<f64>> = Vec::with_capacity(channels as usize);
    for ch in 0..channels as usize {
        let mut v = Vec::with_capacity(frames);
        for i in 0..frames {
            v.push(samples[i * channels as usize + ch] as f64);
        }
        per_channel.push(v);
    }

    // Sample peak
    let mut sample_peak_dbfs = -150.0f64;
    for ch in 0..channels as usize {
        let peak = per_channel[ch].iter().fold(0.0f64, |m, &x| m.max(x.abs()));
        if peak > 1e-6 {
            sample_peak_dbfs = sample_peak_dbfs.max(20.0 * peak.log10());
        }
    }

    // K‑weighting
    let mut kf = KWeighting::new(sr, channels);

    // K‑weighted per‑channel
    let mut kw: Vec<Vec<f64>> = Vec::with_capacity(channels as usize);
    for ch in 0..channels as usize {
        let mut out = Vec::with_capacity(frames);
        for &s in &per_channel[ch] {
            out.push(kf.process(s, ch));
        }
        kw.push(out);
    }

    // Block energies (400 ms, 100 ms hop), summed across channels
    let mut block_energies = Vec::<f64>::new();
    let mut block_loudness = Vec::<f64>::new();

    let mut start = 0usize;
    while start < frames {
        let end = (start + block_400ms).min(frames);
        if end <= start {
            break;
        }

        let mut energy = 0.0f64;
        let len = end - start;

        for ch in 0..channels as usize {
            let slice = &kw[ch][start..end];
            let mut sum_sq = 0.0;
            for &y in slice {
                sum_sq += y * y;
            }
            energy += sum_sq / len as f64;
        }

        block_energies.push(energy);
        if energy > 0.0 {
            block_loudness.push(-0.691 + 10.0 * energy.log10());
        } else {
            block_loudness.push(-150.0);
        }

        if end == frames {
            break;
        }
        start += hop_100ms;
    }

    if block_energies.is_empty() {
        let true_peak_dbtp = compute_true_peak(samples, channels, sample_rate);
        return LoudnessMetrics {
            integrated_lufs: -23.0,
            loudness_range_lu: 0.0,
            true_peak_dbtp,
            sample_peak_dbfs,
        };
    }

    // Absolute gate: −70 LUFS
    let abs_gate_lufs = -70.0;
    let abs_gate_energy = 10.0_f64.powf((abs_gate_lufs + 0.691) / 10.0);

    let gated_indices: Vec<usize> = block_energies
        .iter()
        .enumerate()
        .filter_map(|(i, &e)| if e >= abs_gate_energy { Some(i) } else { None })
        .collect();

    if gated_indices.is_empty() {
        let true_peak_dbtp = compute_true_peak(samples, channels, sample_rate);
        return LoudnessMetrics {
            integrated_lufs: -23.0,
            loudness_range_lu: 0.0,
            true_peak_dbtp,
            sample_peak_dbfs,
        };
    }

    // Ungated integrated loudness over abs‑gated blocks
    let sum_e: f64 = gated_indices.iter().map(|&i| block_energies[i]).sum();
    let mean_e = sum_e / gated_indices.len() as f64;
    let ungated_lufs = -0.691 + 10.0 * mean_e.log10();

    // Relative gate: 10 LU below ungated
    let rel_gate_lufs = ungated_lufs - 10.0;
    let rel_gate_energy = 10.0_f64.powf((rel_gate_lufs + 0.691) / 10.0);

    let final_indices: Vec<usize> = gated_indices
        .into_iter()
        .filter(|&i| block_energies[i] >= rel_gate_energy)
        .collect();

    let integrated_lufs = if final_indices.is_empty() {
        ungated_lufs
    } else {
        let sum_e: f64 = final_indices.iter().map(|&i| block_energies[i]).sum();
        let mean_e = sum_e / final_indices.len() as f64;
        -0.691 + 10.0 * mean_e.log10()
    };

    // LRA: 10th–95th percentile of gated block loudness
    let loudness_range_lu = if final_indices.len() < 2 {
        0.0
    } else {
        let mut vals: Vec<f64> = final_indices.iter().map(|&i| block_loudness[i]).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = vals.len() as f64;
        let p10_pos = 0.10 * (n - 1.0);
        let p95_pos = 0.95 * (n - 1.0);

        let interp = |pos: f64, v: &Vec<f64>| {
            let i = pos.floor() as usize;
            let frac = pos - i as f64;
            if i + 1 < v.len() {
                v[i] * (1.0 - frac) + v[i + 1] * frac
            } else {
                v[i]
            }
        };

        let p10 = interp(p10_pos, &vals);
        let p95 = interp(p95_pos, &vals);
        p95 - p10
    };

    let true_peak_dbtp = compute_true_peak(samples, channels, sample_rate);

    LoudnessMetrics {
        integrated_lufs,
        loudness_range_lu,
        true_peak_dbtp,
        sample_peak_dbfs,
    }
}

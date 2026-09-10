//! flo-fixtures

use anyhow::{anyhow, Context, Result};
use reflo::audio::{write_wav_to_bytes, AudioMetadata};
use reflo::{encode_from_audio, encode_from_samples, EncodeOptions};
use std::f32::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const QUALITY_LOW: f32 = 0.2;
const QUALITY_MEDIUM: f32 = 0.4;
const QUALITY_HIGH: f32 = 0.6;
const QUALITY_VERYHIGH: f32 = 0.8;
const QUALITY_TRANSPARENT: f32 = 1.0;

fn main() -> Result<ExitCode> {
    let mut args = std::env::args_os();
    args.next();
    let examples = PathBuf::from(
        args.next()
            .context("usage: flo-fixtures <EXAMPLES_DIR> <FIXTURES_DIR>")?,
    );
    let fixtures =
        PathBuf::from(args.next().ok_or_else(|| anyhow!("missing <FIXTURES_DIR>"))?);

    regen(&examples, &fixtures)?;
    Ok(ExitCode::SUCCESS)
}

fn regen(examples: &Path, fixtures: &Path) -> Result<()> {
    fs::create_dir_all(fixtures)?;
    fs::create_dir_all(examples)?;

    let bases: &[(&str, Vec<f32>, u32, usize)] = &[
        ("silence_1sec.flo", make_silence(44100, 1.0), 44100, 1),
        ("white_noise.flo", make_noise(44100, 1.0), 44100, 1),
        ("sine_440hz_mono.flo", make_sine(44100, 440.0, 2.0, 0.5), 44100, 1),
        (
            "chord_cmajor_stereo.flo",
            interleave(&make_sines(44100, &[261.63, 329.63, 392.0], 2.0, 0.5), 2),
            44100,
            2,
        ),
        (
            "sweep_20_20k.flo",
            make_square_sweep(44100, 20.0, 20_000.0, 5.0, 0.25),
            44100,
            1,
        ),
        ("hires_96khz.flo", make_sine(96_000, 1000.0, 1.0, 0.5), 96_000, 1),
        ("telephone_8khz.flo", make_sine(8_000, 1000.0, 1.0, 0.5), 8_000, 1),
        (
            "click_track_120bpm.flo",
            make_sine(44100, 1000.0, 0.05, 0.5),
            44100,
            1,
        ),
        (
            "multitone_stereo.flo",
            interleave(&make_sines(44100, &[440.0, 880.0], 2.0, 0.5), 2),
            44100,
            2,
        ),
        (
            "dtmf_tones.flo",
            make_sines(44100, &[697.0, 1209.0], 0.2, 0.5),
            44100,
            1,
        ),
    ];

    for (name, samples, rate, channels) in bases {
        write_fixture(
            name,
            samples,
            *rate,
            *channels,
            EncodeOptions::lossless(),
            examples,
            fixtures,
        )?;
    }

    let chord = make_sines(44100, &[261.63, 329.63, 392.0], 2.0, 0.5);
    let chord_stereo = interleave(&chord, 2);

    for (name, quality) in [
        ("lossy_chord_low.flo", QUALITY_LOW),
        ("lossy_chord_medium.flo", QUALITY_MEDIUM),
        ("lossy_chord_high.flo", QUALITY_HIGH),
        ("lossy_chord_veryhigh.flo", QUALITY_VERYHIGH),
        ("lossy_chord_transparent.flo", QUALITY_TRANSPARENT),
    ] {
        write_fixture(
            name,
            &chord_stereo,
            44100,
            2,
            EncodeOptions::lossy(quality),
            examples,
            fixtures,
        )?;
    }

    write_fixture(
        "lossy_music_pattern.flo",
        &make_arpeggio(44100, &[261.63, 329.63, 392.0, 493.88, 523.25], 0.5, 0.5),
        44100,
        1,
        EncodeOptions::lossy(QUALITY_HIGH),
        examples,
        fixtures,
    )?;

    let src = examples.join("audio.wav");
    if src.exists() {
        let bytes = fs::read(&src).with_context(|| format!("reading {}", src.display()))?;
        let lossless = encode_from_audio(&bytes, EncodeOptions::lossless())
            .context("encoding audio_lossless.flo")?;
        write_flo("audio_lossless.flo", &lossless, examples, fixtures);
        let lossy = encode_from_audio(&bytes, EncodeOptions::lossy(QUALITY_HIGH))
            .context("encoding audio_lossy.flo")?;
        write_flo("audio_lossy.flo", &lossy, examples, fixtures);
    }

    println!(
        "[flo-fixtures] regenerated {} -> {} and {} -> {}",
        fixtures.display(),
        counts(fixtures),
        examples.display(),
        counts(examples),
    );
    Ok(())
}

fn counts(dir: &Path) -> String {
    match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).count().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn write_fixture(
    name: &str,
    samples: &[f32],
    rate: u32,
    channels: usize,
    options: EncodeOptions,
    examples: &Path,
    fixtures: &Path,
) -> Result<()> {
    let wav = write_wav_to_bytes(samples, rate, channels).context("writing WAV bytes")?;
    let flo = encode_from_samples(samples, rate, channels, AudioMetadata::default(), options)
        .with_context(|| format!("encoding {name}"))?;

    let stem = name.strip_suffix(".flo").unwrap_or(name);
    fs::write(fixtures.join(format!("{stem}.wav")), wav)
        .with_context(|| format!("writing {stem}.wav to fixtures"))?;
    write_flo(name, &flo, examples, fixtures);
    Ok(())
}

fn write_flo(name: &str, flo: &[u8], examples: &Path, fixtures: &Path) {
    let _ = fs::write(fixtures.join(name), flo);
    let _ = fs::write(examples.join(name), flo);
}

// Signal synthesis
// All generators produce mono sample vectors in [-1.0, 1.0].

fn make_sine(rate: u32, freq: f32, seconds: f64, amp: f32) -> Vec<f32> {
    let frames = (rate as f64 * seconds).round() as usize;
    (0..frames)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (2.0 * PI * freq * t).sin() * amp
        })
        .collect()
}

fn make_sines(rate: u32, freqs: &[f32], seconds: f64, amp: f32) -> Vec<f32> {
    let frames = (rate as f64 * seconds).round() as usize;
    let per = amp / freqs.len() as f32;
    (0..frames)
        .map(|i| {
            let t = i as f32 / rate as f32;
            freqs.iter().map(|f| (2.0 * PI * f * t).sin()).sum::<f32>() * per
        })
        .collect()
}

fn make_arpeggio(rate: u32, notes: &[f32], note_secs: f32, amp: f32) -> Vec<f32> {
    let per_note = (rate as f32 * note_secs).round() as usize;
    let mut out = Vec::with_capacity(per_note * notes.len());
    for &f in notes {
        for i in 0..per_note {
            let t = i as f32 / rate as f32;
            out.push((2.0 * PI * f * t).sin() * amp);
        }
    }
    out
}

fn make_square_sweep(rate: u32, f0: f32, f1: f32, seconds: f64, amp: f32) -> Vec<f32> {
    let frames = (rate as f64 * seconds).round() as usize;
    let mut out = Vec::with_capacity(frames);
    let mut phase = 0.0f64;
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let f = f0 + (f1 - f0) * (t / seconds as f32);
        phase += 2.0 * PI as f64 * f as f64 / rate as f64;
        out.push(if phase.sin() >= 0.0 { amp } else { -amp });
    }
    out
}

fn make_silence(rate: u32, seconds: f64) -> Vec<f32> {
    vec![0.0; (rate as f64 * seconds).round() as usize]
}

fn make_noise(rate: u32, seconds: f64) -> Vec<f32> {
    let frames = (rate as f64 * seconds).round() as usize;
    let mut out = Vec::with_capacity(frames);
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    for _ in 0..frames {
        // xorshift64*
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as u64;
        // Map the low 32 bits onto [-1.0, 1.0].
        let v = (r as f32 / (1u64 << 31) as f32) - 1.0;
        out.push(v);
    }
    out
}

fn interleave(mono: &[f32], channels: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(mono.len() * channels);
    for &s in mono {
        for _ in 0..channels {
            out.push(s);
        }
    }
    out
}
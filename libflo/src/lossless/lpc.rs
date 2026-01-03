/// Calculate autocorrelation coefficients
pub fn autocorrelation(samples: &[f32], max_lag: usize) -> Vec<f32> {
    let n = samples.len();
    let mut autocorr = vec![0.0; max_lag + 1];

    for lag in 0..=max_lag {
        let mut sum = 0.0;
        for i in 0..(n.saturating_sub(lag)) {
            sum += samples[i] * samples[i + lag];
        }
        autocorr[lag] = sum;
    }

    autocorr
}

/// Levinson-Durbin algorithm for LPC coefficient calculation
pub fn levinson_durbin(autocorr: &[f32], order: usize) -> Vec<f32> {
    if order == 0 || autocorr.is_empty() {
        return vec![];
    }

    let mut coeffs = vec![0.0; order];
    let mut prev = vec![0.0; order];
    let mut error = autocorr[0];

    if error.abs() < 1e-10 {
        error = 1e-10;
    }

    for i in 0..order {
        let mut lambda = autocorr.get(i + 1).copied().unwrap_or(0.0);
        for j in 0..i {
            lambda -= coeffs[j] * autocorr.get(i - j).copied().unwrap_or(0.0);
        }
        lambda /= error;
        lambda = lambda.clamp(-0.999, 0.999);

        prev.copy_from_slice(&coeffs);

        coeffs[i] = lambda;
        for j in 0..i {
            coeffs[j] = prev[j] - lambda * prev[i - 1 - j];
        }

        error *= 1.0 - lambda * lambda;
        if error.abs() < 1e-10 {
            error = 1e-10;
        }
    }

    coeffs
}

/// Calculate prediction residuals
pub fn calculate_residuals(samples: &[f32], coeffs: &[f32]) -> Vec<f32> {
    let order = coeffs.len();
    let mut residuals = Vec::with_capacity(samples.len());

    // First 'order' samples stored as-is (warm-up)
    for i in 0..order.min(samples.len()) {
        residuals.push(samples[i]);
    }

    // Remaining samples: residual = actual - predicted
    for i in order..samples.len() {
        let mut prediction = 0.0;
        for (j, &coeff) in coeffs.iter().enumerate() {
            prediction += coeff * samples[i - j - 1];
        }
        residuals.push(samples[i] - prediction);
    }

    residuals
}

/// Reconstruct samples from coefficients and residuals
pub fn reconstruct_samples(coeffs: &[f32], residuals: &[f32], target_len: usize) -> Vec<f32> {
    let order = coeffs.len();
    let mut samples = Vec::with_capacity(target_len);

    // First 'order' samples from residuals (warm-up)
    for i in 0..order.min(residuals.len()) {
        samples.push(residuals[i]);
    }

    // Reconstruct remaining: sample = prediction + residual
    for i in order..target_len.min(residuals.len()) {
        let mut prediction = 0.0;
        for (j, &coeff) in coeffs.iter().enumerate() {
            if i > j {
                prediction += coeff * samples[i - j - 1];
            }
        }
        samples.push(prediction + residuals[i]);
    }

    // Pad with zeros if needed
    while samples.len() < target_len {
        samples.push(0.0);
    }

    samples
}

/// Quantize floating-point coefficients to integers
pub fn quantize_coefficients(coeffs: &[f32]) -> (Vec<i32>, u8) {
    if coeffs.is_empty() {
        return (vec![], 0);
    }

    let max_val = coeffs.iter().map(|&c| c.abs()).fold(0.0f32, f32::max);

    let shift_bits = if max_val > 0.0 && max_val.is_finite() {
        let ratio = 2147483647.0f32 / max_val;
        if ratio > 1.0 {
            (ratio.log2().floor() as i32).clamp(0, 28) as u8
        } else {
            0
        }
    } else {
        15
    };

    let scale = if shift_bits < 31 {
        (1u32 << shift_bits) as f32
    } else {
        2147483648.0
    };
    let quantized: Vec<i32> = coeffs.iter().map(|&c| (c * scale).round() as i32).collect();

    (quantized, shift_bits)
}

/// Dequantize integer coefficients back to floats
pub fn dequantize_coefficients(coeffs: &[i32], shift_bits: u8) -> Vec<f32> {
    let scale = if shift_bits < 31 {
        1.0 / (1u32 << shift_bits) as f32
    } else {
        1.0 / 2147483648.0
    };
    coeffs.iter().map(|&c| c as f32 * scale).collect()
}

/// Check if LPC coefficients represent a stable filter
/// A filter is stable if all poles are inside the unit circle.
/// This approximation checks if the sum of absolute coefficients is reasonable.
pub fn is_stable(coeffs: &[f32]) -> bool {
    if coeffs.is_empty() {
        return true;
    }

    // Check 1: No coefficient should be too large (absolute value)
    let max_coef = coeffs.iter().map(|c| c.abs()).fold(0.0f32, f32::max);
    if max_coef > 1.5 {
        return false;
    }

    // Check 2: Sum of absolute values shouldn't exceed order
    // For a stable filter, sum should be less than 1 for IIR stability
    let sum_abs: f32 = coeffs.iter().map(|c| c.abs()).sum();
    if sum_abs > coeffs.len() as f32 {
        return false;
    }

    // Check 3: Simulate a few steps of the filter to detect divergence
    // Feed it an impulse and see if the output stays bounded
    let test_len = 50.max(coeffs.len() * 5);
    let mut output = vec![0.0f32; test_len];
    output[0] = 1.0; // impulse

    for i in 1..test_len {
        let mut val = 0.0;
        for (j, &coeff) in coeffs.iter().enumerate() {
            if i > j {
                val += coeff * output[i - j - 1];
            }
        }
        output[i] = val;

        // If output grows at all beyond initial impulse, filter is marginally unstable (haha)
        if val.abs() > 2.0 || !val.is_finite() {
            return false;
        }
    }

    true
}

/// Check stability after quantization roundtrip (more strict)
pub fn is_stable_after_quantization(coeffs: &[f32]) -> bool {
    if coeffs.is_empty() {
        return true;
    }

    // First check raw stability
    if !is_stable(coeffs) {
        return false;
    }

    // Quantize and dequantize to see if roundtrip is still stable
    let (quantized, shift) = quantize_coefficients(coeffs);
    let recovered = dequantize_coefficients(&quantized, shift);

    is_stable(&recovered)
}

// ============================================================================
// Integer LPC functions
// ============================================================================

/// Integer autocorrelation
pub fn autocorr_int(samples: &[i32], order: usize) -> Vec<i64> {
    let mut autocorr = vec![0i64; order + 1];
    for lag in 0..=order {
        for i in lag..samples.len() {
            autocorr[lag] += (samples[i] as i64) * (samples[i - lag] as i64);
        }
    }
    autocorr
}

/// Levinson-Durbin in fixed-point
/// Returns coefficients scaled by 2^shift and the shift value
pub fn levinson_durbin_int(autocorr: &[i64], order: usize) -> Option<(Vec<i32>, u8)> {
    if autocorr.is_empty() || autocorr[0] == 0 {
        return None;
    }

    // Work in f64 for precision, then convert to fixed-point
    let mut coeffs = vec![0.0f64; order];
    let mut error = autocorr[0] as f64;

    for i in 0..order {
        let mut lambda = autocorr.get(i + 1).copied().unwrap_or(0) as f64;
        for j in 0..i {
            lambda -= coeffs[j] * autocorr.get(i - j).copied().unwrap_or(0) as f64;
        }

        if error.abs() < 1e-10 {
            return None;
        }

        let gamma = lambda / error;

        // Check for instability
        if gamma.abs() >= 1.0 {
            return None;
        }

        let mut new_coeffs = vec![0.0f64; i + 1];
        new_coeffs[i] = gamma;
        for j in 0..i {
            new_coeffs[j] = coeffs[j] - gamma * coeffs[i - 1 - j];
        }
        coeffs[..=i].copy_from_slice(&new_coeffs);

        error *= 1.0 - gamma * gamma;
    }

    // Convert to fixed-point
    // Find appropriate shift to maximize precision
    let max_coeff = coeffs.iter().map(|&c| c.abs()).fold(0.0f64, f64::max);
    if max_coeff == 0.0 || !max_coeff.is_finite() {
        return None;
    }

    // Use shift that keeps coefficients in i32 range with good precision
    let shift = ((1 << 30) as f64 / max_coeff).log2().floor() as u8;
    let shift = shift.min(15); // Cap at 15 bits
    let scale = (1i64 << shift) as f64;

    let coeffs_fp: Vec<i32> = coeffs.iter().map(|&c| (c * scale).round() as i32).collect();

    Some((coeffs_fp, shift))
}

/// Calculate residuals using integer predictor
pub fn calc_residuals_int(samples: &[i32], coeffs: &[i32], shift: u8, order: usize) -> Vec<i32> {
    let mut residuals = Vec::with_capacity(samples.len());

    // Warmup samples
    for i in 0..order.min(samples.len()) {
        residuals.push(samples[i]);
    }

    // Predicted samples
    for i in order..samples.len() {
        let mut prediction: i64 = 0;
        for (j, &coeff) in coeffs.iter().enumerate() {
            prediction += (coeff as i64) * (samples[i - j - 1] as i64);
        }
        prediction >>= shift;
        residuals.push(samples[i] - prediction as i32);
    }

    residuals
}

/// Fixed predictor residuals
pub fn fixed_predictor_residuals(samples: &[i32], order: usize) -> Vec<i32> {
    match order {
        0 => samples.to_vec(),
        1 => {
            // First-order: r[i] = s[i] - s[i-1]
            let mut r = vec![samples[0]];
            for i in 1..samples.len() {
                r.push(samples[i] - samples[i - 1]);
            }
            r
        }
        2 => {
            // Second-order: r[i] = s[i] - 2*s[i-1] + s[i-2]
            let mut r = vec![samples[0]];
            if samples.len() > 1 {
                r.push(samples[1] - samples[0]);
            }
            for i in 2..samples.len() {
                r.push(samples[i] - 2 * samples[i - 1] + samples[i - 2]);
            }
            r
        }
        3 => {
            // Third-order
            let mut r = vec![samples[0]];
            if samples.len() > 1 {
                r.push(samples[1] - samples[0]);
            }
            if samples.len() > 2 {
                r.push(samples[2] - 2 * samples[1] + samples[0]);
            }
            for i in 3..samples.len() {
                r.push(samples[i] - 3 * samples[i - 1] + 3 * samples[i - 2] - samples[i - 3]);
            }
            r
        }
        4 => {
            // Fourth-order
            let mut r = vec![samples[0]];
            if samples.len() > 1 {
                r.push(samples[1] - samples[0]);
            }
            if samples.len() > 2 {
                r.push(samples[2] - 2 * samples[1] + samples[0]);
            }
            if samples.len() > 3 {
                r.push(samples[3] - 3 * samples[2] + 3 * samples[1] - samples[0]);
            }
            for i in 4..samples.len() {
                r.push(
                    samples[i] - 4 * samples[i - 1] + 6 * samples[i - 2] - 4 * samples[i - 3]
                        + samples[i - 4],
                );
            }
            r
        }
        _ => samples.to_vec(),
    }
}

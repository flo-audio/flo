//! Example: Convert an audio file to flo format and back
//!
//! Run with: cargo run --example convert_audio input.mp3 output.flo

use reflo::{decode_to_wav, encode_from_audio, get_flo_info, EncodeOptions};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input-audio> <output-flo>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Reading {}...", input_path);
    let audio_bytes = fs::read(input_path)?;

    // Get audio info
    let info = reflo::get_audio_info(&audio_bytes)?;
    println!("  Sample rate: {} Hz", info.sample_rate);
    println!("  Channels: {}", info.channels);
    println!("  Duration: {:.2}s", info.duration_secs);

    // Encode with lossy compression (high quality)
    println!("\nEncoding to flo (lossy, high quality)...");
    let options = EncodeOptions::lossy(0.6) // 0.0 = low, 1.0 = transparent
        .with_level(5);

    let flo_bytes = encode_from_audio(&audio_bytes, options)?;

    // Show compression stats
    let original_size = audio_bytes.len();
    let compressed_size = flo_bytes.len();
    let ratio = original_size as f32 / compressed_size as f32;

    println!("  Original: {} bytes", original_size);
    println!("  Compressed: {} bytes", compressed_size);
    println!("  Ratio: {:.1}x", ratio);

    // Write to file
    fs::write(output_path, &flo_bytes)?;
    println!("\nWrote flo file to {}", output_path);

    // Get flo file info
    let flo_info =
        get_flo_info(&flo_bytes).map_err(|e| anyhow::anyhow!("Failed to get flo info: {:?}", e))?;
    println!("\nflo File Info:");
    println!("  Sample rate: {} Hz", flo_info.sample_rate);
    println!("  Channels: {}", flo_info.channels);
    println!("  Duration: {:.2}s", flo_info.duration_secs);
    println!("  Lossy: {}", if flo_info.is_lossy { "yes" } else { "no" });
    println!(
        "  CRC valid: {}",
        if flo_info.crc_valid { "yes" } else { "no" }
    );

    // Decode back to WAV for verification
    println!("\nDecoding back to WAV for verification...");
    let wav_bytes = decode_to_wav(&flo_bytes)?;
    let wav_path = output_path.replace(".flo", "_decoded.wav");
    fs::write(&wav_path, wav_bytes)?;
    println!("Wrote decoded WAV to {}", wav_path);

    Ok(())
}

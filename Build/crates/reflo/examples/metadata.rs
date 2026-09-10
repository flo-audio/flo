//! Example: Extract and display metadata from a flo file
//!
//! Run with: cargo run --example metadata audio.flo

use reflo::get_metadata;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <flo-file>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];

    println!("Reading {}...", input_path);
    let flo_bytes = fs::read(input_path)?;

    match get_metadata(&flo_bytes)? {
        Some(meta) => {
            println!("\nMetadata:");
            println!("═══════════════════════════════════");

            if let Some(title) = &meta.title {
                println!("Title:       {}", title);
            }
            if let Some(artist) = &meta.artist {
                println!("Artist:      {}", artist);
            }
            if let Some(album) = &meta.album {
                println!("Album:       {}", album);
            }
            if let Some(year) = meta.year {
                println!("Year:        {}", year);
            }
            if let Some(genre) = &meta.genre {
                println!("Genre:       {}", genre);
            }
            if let Some(bpm) = meta.bpm {
                println!("BPM:         {}", bpm);
            }

            if !meta.pictures.is_empty() {
                println!("\nCover Art:");
                for (i, pic) in meta.pictures.iter().enumerate() {
                    println!(
                        "  [{}] {:?} - {} ({} bytes)",
                        i + 1,
                        pic.picture_type,
                        pic.mime_type,
                        pic.data.len()
                    );
                }
            }

            if !meta.synced_lyrics.is_empty() {
                println!("\nSynced Lyrics: {} track(s)", meta.synced_lyrics.len());
            }

            // Convert to JSON
            println!("\nJSON representation:");
            let json = serde_json::to_string_pretty(&meta)?;
            println!("{}", json);
        }
        None => {
            println!("No metadata found in file");
        }
    }

    Ok(())
}

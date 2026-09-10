use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use reflo::{EncodeOptions, FloMetadata};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "flo")]
#[command(author = "NellowTCS")]
#[command(version = "0.1.2")]
#[command(about = "flo™ audio format converter", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode audio file to flo™ format
    Encode {
        /// Input audio file (mp3, wav, flac, ogg, etc.)
        input: PathBuf,
        /// Output flo™ file
        output: PathBuf,
        /// Compression level (0-9, default 5)
        #[arg(short, long, default_value = "5")]
        level: u8,
        /// Enable lossy compression mode
        #[arg(long)]
        lossy: bool,
        /// Use transform-based lossy
        #[arg(long)]
        transform: bool,
        /// Lossy quality level (low, medium, high, veryhigh, transparent)
        #[arg(long, default_value = "high")]
        quality: String,
        /// Target bitrate in kbps (alternative to quality)
        #[arg(long)]
        bitrate: Option<u32>,
        /// Title metadata
        #[arg(long)]
        title: Option<String>,
        /// Artist metadata
        #[arg(long)]
        artist: Option<String>,
        /// Album metadata
        #[arg(long)]
        album: Option<String>,
    },
    /// Decode flo™ file to WAV
    Decode {
        /// Input flo™ file
        input: PathBuf,
        /// Output WAV file
        output: PathBuf,
    },
    /// Show information about a flo™ file
    Info {
        /// Input flo™ file
        input: PathBuf,
        /// Show metadata details
        #[arg(short, long)]
        metadata: bool,
    },
    /// Display metadata from a flo™ file
    Metadata {
        /// Input flo™ file
        input: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Analyze audio content (loudness, waveform, spectrum)
    Analysis {
        /// Input flo™ file
        input: PathBuf,
        /// Show waveform peaks
        #[arg(short, long)]
        waveform: bool,
        /// Show spectral fingerprint
        #[arg(short, long)]
        spectrum: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Validate a flo™ file
    Validate {
        /// Input flo™ file
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            input,
            output,
            level,
            lossy,
            transform,
            quality,
            bitrate,
            title,
            artist,
            album,
        } => {
            // Both --lossy and --transform enable lossy mode
            let use_lossy = lossy || transform;
            encode(EncodeArgs {
                input,
                output,
                level,
                lossy: use_lossy,
                quality,
                bitrate,
                title,
                artist,
                album,
            })?;
        }
        Commands::Decode { input, output } => {
            decode(&input, &output)?;
        }
        Commands::Info {
            input,
            metadata: show_meta,
        } => {
            info(&input, show_meta)?;
        }
        Commands::Metadata { input, json } => {
            metadata(&input, json)?;
        }
        Commands::Analysis {
            input,
            waveform,
            spectrum,
            json,
        } => {
            analysis(&input, waveform, spectrum, json)?;
        }
        Commands::Validate { input } => {
            validate(&input)?;
        }
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct AnalysisOutput {
    file_info: FileInfo,
    loudness: LoudnessOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    waveform: Option<WaveformOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spectral: Option<SpectralOutput>,
}

#[derive(Serialize, Deserialize)]
struct FileInfo {
    sample_rate: u32,
    channels: u8,
    bit_depth: u8,
    duration_secs: f64,
    total_samples: u64,
}

#[derive(Serialize, Deserialize)]
struct LoudnessOutput {
    integrated_lufs: f64,
    loudness_range_lu: f64,
    true_peak_dbtp: f64,
    sample_peak_dbfs: f64,
}

#[derive(Serialize, Deserialize)]
struct WaveformOutput {
    peaks_per_second: u32,
    total_peaks: usize,
    channels: u8,
    peak_statistics: Option<PeakStats>,
}

#[derive(Serialize, Deserialize)]
struct PeakStats {
    min: f32,
    max: f32,
    average: f32,
}

#[derive(Serialize, Deserialize)]
struct SpectralOutput {
    duration_ms: u32,
    sample_rate: u32,
    channels: u8,
    peak_frequency_bands: Vec<u8>,
    energy_profile: Vec<u8>,
    average_loudness: u8,
    spectral_hash_hex: String,
}

struct EncodeArgs {
    input: PathBuf,
    output: PathBuf,
    level: u8,
    lossy: bool,
    quality: String,
    bitrate: Option<u32>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
}

fn encode(args: EncodeArgs) -> Result<()> {
    println!("Reading {}...", args.input.display());

    // Read audio file
    let audio_bytes = fs::read(&args.input).context("Failed to read input file")?;

    let info = reflo::get_audio_info(&audio_bytes).context("Failed to read audio file")?;

    println!("  Sample rate: {} Hz", info.sample_rate);
    println!("  Channels: {}", info.channels);
    println!("  Duration: {:.2}s", info.duration_secs);

    // Build encoding options
    let mut options = if args.lossy || args.bitrate.is_some() {
        if let Some(br) = args.bitrate {
            println!("Encoding to flo™ (lossy, ~{} kbps)...", br);
            EncodeOptions::lossy_bitrate(br)
        } else {
            let quality_value = match args.quality.to_lowercase().as_str() {
                "low" => 0.2,
                "medium" | "med" => 0.4,
                "high" => 0.6,
                "veryhigh" | "vh" => 0.8,
                "transparent" | "trans" => 1.0,
                _ => bail!(
                    "Invalid quality level: {}. Use: low, medium, high, veryhigh, transparent",
                    args.quality
                ),
            };
            println!("Encoding to flo™ (lossy, {} quality)...", args.quality);
            EncodeOptions::lossy(quality_value)
        }
    } else {
        println!("Encoding to flo™ (lossless)...");
        EncodeOptions::lossless()
    };

    options = options.with_level(args.level);

    // Add metadata if provided via CLI
    if args.title.is_some() || args.artist.is_some() || args.album.is_some() {
        let mut meta = FloMetadata::new();
        if let Some(t) = args.title {
            meta.title = Some(t);
        }
        if let Some(a) = args.artist {
            meta.artist = Some(a);
        }
        if let Some(a) = args.album {
            meta.album = Some(a);
        }
        options = options.with_metadata(meta);
    }

    // Encode
    let flo_data =
        reflo::encode_from_audio(&audio_bytes, options).context("Failed to encode audio")?;

    fs::write(&args.output, &flo_data).context("Failed to write output file")?;

    let original_size =
        (info.sample_rate as f32 * info.channels as f32 * info.duration_secs * 4.0) as usize;
    let compressed_size = flo_data.len();
    let ratio = original_size as f32 / compressed_size as f32;

    println!("Done!");
    println!("  Output: {}", args.output.display());
    println!(
        "  Size: {} bytes ({:.1}x compression)",
        compressed_size, ratio
    );

    Ok(())
}

fn decode(input: &PathBuf, output: &PathBuf) -> Result<()> {
    println!("Reading {}...", input.display());

    let flo_data = fs::read(input).context("Failed to read flo™ file")?;

    // Get info first
    let file_info =
        reflo::get_flo_info(&flo_data).map_err(|_| anyhow::anyhow!("Invalid flo™ file"))?;

    println!("  Sample rate: {} Hz", file_info.sample_rate);
    println!("  Channels: {}", file_info.channels);
    println!("  Duration: {:.2}s", file_info.duration_secs);

    println!("Decoding...");

    let wav_bytes = reflo::decode_to_wav(&flo_data).context("Failed to decode flo™ file")?;

    println!("Writing WAV...");

    fs::write(output, wav_bytes).context("Failed to write WAV file")?;

    println!("Done!");
    println!("  Output: {}", output.display());

    Ok(())
}

fn info(input: &PathBuf, show_metadata: bool) -> Result<()> {
    let flo_data = fs::read(input).context("Failed to read flo™ file")?;

    let file_info =
        reflo::get_flo_info(&flo_data).map_err(|_| anyhow::anyhow!("Invalid flo™ file"))?;

    println!("flo™ Audio File");
    println!("───────────────────────────────");
    println!("  Version:     {}", file_info.version);
    println!("  Sample rate: {} Hz", file_info.sample_rate);
    println!("  Channels:    {}", file_info.channels);
    println!("  Bit depth:   {}", file_info.bit_depth);
    println!("  Duration:    {:.2}s", file_info.duration_secs);
    println!("  Total sample-frames: {}", file_info.total_samples);
    println!("  File size:   {} bytes", file_info.file_size);
    println!("  Compression: {:.1}x", file_info.compression_ratio);
    println!(
        "  CRC valid:   {}",
        if file_info.crc_valid { "yes" } else { "no" }
    );

    // Show encoding mode
    if file_info.is_lossy {
        let quality_names = ["Low", "Medium", "High", "VeryHigh", "Transparent"];
        let quality_name = quality_names
            .get(file_info.lossy_quality as usize)
            .unwrap_or(&"Unknown");
        println!("  Encoding:    Lossy ({})", quality_name);
    } else {
        println!("  Encoding:    Lossless");
    }

    if show_metadata {
        println!();
        println!("Metadata");
        println!("───────────────────────────────");

        // Try to read metadata
        if let Ok(Some(meta)) = reflo::get_metadata(&flo_data) {
            if let Some(ref title) = meta.title {
                println!("  Title:       {}", title);
            }
            if let Some(ref artist) = meta.artist {
                println!("  Artist:      {}", artist);
            }
            if let Some(ref album) = meta.album {
                println!("  Album:       {}", album);
            }
            if let Some(year) = meta.year {
                println!("  Year:        {}", year);
            }
            if let Some(ref genre) = meta.genre {
                println!("  Genre:       {}", genre);
            }
            if let Some(bpm) = meta.bpm {
                println!("  BPM:         {}", bpm);
            }
            if let Some(ref key) = meta.key {
                println!("  Key:         {}", key);
            }
            if !meta.pictures.is_empty() {
                println!("  Pictures:    {} attached", meta.pictures.len());
            }
            if !meta.synced_lyrics.is_empty() {
                println!("  Synced lyrics: yes");
            }
            if !meta.section_markers.is_empty() {
                println!("  Sections:    {} markers", meta.section_markers.len());
            }
            if meta.waveform_data.is_some() {
                println!("  Waveform:    pre-computed");
            }
        } else {
            println!("  (no metadata)");
        }
    }

    Ok(())
}

fn metadata(input: &PathBuf, json: bool) -> Result<()> {
    let flo_data = fs::read(input).context("Failed to read flo™ file")?;

    match reflo::get_metadata(&flo_data)? {
        None => {
            if json {
                println!("null");
            } else {
                println!("No metadata present");
            }
            Ok(())
        }
        Some(meta) => {
            if json {
                let json_str =
                    serde_json::to_string_pretty(&meta).context("Failed to serialize metadata")?;
                println!("{}", json_str);
            } else {
                // Human-readable format
                print_metadata_readable(&meta);
            }
            Ok(())
        }
    }
}

fn print_metadata_readable(meta: &FloMetadata) {
    println!("flo™ Metadata");
    println!("═══════════════════════════════════════");

    // Identification
    if let Some(ref v) = meta.title {
        println!("Title:           {}", v);
    }
    if let Some(ref v) = meta.subtitle {
        println!("Subtitle:        {}", v);
    }
    if let Some(ref v) = meta.album {
        println!("Album:           {}", v);
    }
    if let Some(v) = meta.track_number {
        if let Some(total) = meta.track_total {
            println!("Track:           {}/{}", v, total);
        } else {
            println!("Track:           {}", v);
        }
    }
    if let Some(v) = meta.disc_number {
        if let Some(total) = meta.disc_total {
            println!("Disc:            {}/{}", v, total);
        } else {
            println!("Disc:            {}", v);
        }
    }
    if let Some(ref v) = meta.isrc {
        println!("ISRC:            {}", v);
    }

    // People
    if let Some(ref v) = meta.artist {
        println!("Artist:          {}", v);
    }
    if let Some(ref v) = meta.album_artist {
        println!("Album Artist:    {}", v);
    }
    if let Some(ref v) = meta.composer {
        println!("Composer:        {}", v);
    }
    if let Some(ref v) = meta.conductor {
        println!("Conductor:       {}", v);
    }
    if let Some(ref v) = meta.lyricist {
        println!("Lyricist:        {}", v);
    }
    if let Some(ref v) = meta.remixer {
        println!("Remixer:         {}", v);
    }

    // Properties
    if let Some(ref v) = meta.genre {
        println!("Genre:           {}", v);
    }
    if let Some(ref v) = meta.mood {
        println!("Mood:            {}", v);
    }
    if let Some(v) = meta.year {
        println!("Year:            {}", v);
    }
    if let Some(v) = meta.bpm {
        println!("BPM:             {}", v);
    }
    if let Some(ref v) = meta.key {
        println!("Key:             {}", v);
    }
    if let Some(ref v) = meta.language {
        println!("Language:        {}", v);
    }

    // Loudness
    if meta.integrated_loudness_lufs.is_some() || meta.true_peak_dbtp.is_some() {
        println!();
        println!("Loudness");
        println!("───────────────────────────────────────");
        if let Some(v) = meta.integrated_loudness_lufs {
            println!("Integrated:      {:.1} LUFS", v);
        }
        if let Some(v) = meta.loudness_range_lu {
            println!("Range:           {:.1} LU", v);
        }
        if let Some(v) = meta.true_peak_dbtp {
            println!("True Peak:       {:.1} dBTP", v);
        }
    }

    // Complex fields
    if !meta.pictures.is_empty() {
        println!();
        println!("Pictures ({}):", meta.pictures.len());
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
        println!();
        println!("Synced Lyrics ({} tracks):", meta.synced_lyrics.len());
        for sylt in &meta.synced_lyrics {
            let lang = sylt.language.as_deref().unwrap_or("und");
            println!(
                "  [{}] {:?} - {} lines",
                lang,
                sylt.content_type,
                sylt.lines.len()
            );
        }
    }

    if !meta.section_markers.is_empty() {
        println!();
        println!("Sections ({}):", meta.section_markers.len());
        for sec in &meta.section_markers {
            let time = format_time(sec.timestamp_ms);
            let label = sec.label.as_deref().unwrap_or("");
            println!("  {} {:?} {}", time, sec.section_type, label);
        }
    }

    if !meta.bpm_map.is_empty() {
        println!();
        println!("BPM Map ({} changes):", meta.bpm_map.len());
        for bpm in &meta.bpm_map {
            println!("  {} - {:.1} BPM", format_time(bpm.timestamp_ms), bpm.bpm);
        }
    }

    if !meta.key_changes.is_empty() {
        println!();
        println!("Key Changes ({}):", meta.key_changes.len());
        for kc in &meta.key_changes {
            println!("  {} - {}", format_time(kc.timestamp_ms), kc.key);
        }
    }

    if let Some(wd) = meta.waveform_data.as_ref() {
        println!();
        println!("Waveform Data:");
        println!("  Peaks/sec:     {}", wd.peaks_per_second);
        println!("  Total peaks:   {}", wd.peaks.len());
        println!("  Channels:      {}", wd.channels);
    }

    if !meta.collaboration_credits.is_empty() {
        println!();
        println!("Collaboration Credits:");
        for cred in &meta.collaboration_credits {
            println!("  {} - {}", cred.role, cred.name);
        }
    }

    if !meta.creator_notes.is_empty() {
        println!();
        println!("Creator Notes ({}):", meta.creator_notes.len());
        for note in &meta.creator_notes {
            if let Some(ts) = note.timestamp_ms {
                println!("  {} - {}", format_time(ts), note.text);
            } else {
                println!("  {}", note.text);
            }
        }
    }

    // flo-specific
    if meta.flo_encoder_version.is_some() || meta.source_format.is_some() {
        println!();
        println!("flo™ Info");
        println!("───────────────────────────────────────");
        if let Some(ref v) = meta.flo_encoder_version {
            println!("Encoder:         {}", v);
        }
        if let Some(ref v) = meta.source_format {
            println!("Source:          {}", v);
        }
    }
}

fn format_time(ms: u64) -> String {
    let secs = ms / 1000;
    let mins = secs / 60;
    let secs = secs % 60;
    let ms_rem = ms % 1000;
    format!("{:02}:{:02}.{:03}", mins, secs, ms_rem)
}

fn analysis(
    input: &PathBuf,
    show_waveform: bool,
    show_spectrum: bool,
    output_json: bool,
) -> Result<()> {
    let flo_data = fs::read(input).context("Failed to read flo™ file")?;

    // Get file info
    let file_info =
        reflo::get_flo_info(&flo_data).map_err(|_| anyhow::anyhow!("Invalid flo™ file"))?;

    // Decode directly to samples
    let (samples, _sample_rate, _channels) =
        reflo::decode_to_samples(&flo_data).context("Failed to decode flo™ file")?;

    // Loudness Analysis using libflo EBU R128 metrics
    let loudness = libflo_audio::compute_ebu_r128_loudness(
        &samples,
        file_info.channels,
        file_info.sample_rate,
    );

    // Prepare waveform data if requested
    let waveform_data = if show_waveform {
        let waveform = libflo_audio::extract_waveform_peaks(
            &samples,
            file_info.channels,
            file_info.sample_rate,
            60, // 60 peaks per second for visualization
        );

        let peak_stats = if !waveform.peaks.is_empty() {
            let peak_min = waveform.peaks.iter().copied().fold(f32::INFINITY, f32::min);
            let peak_max = waveform
                .peaks
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            let peak_avg = waveform.peaks.iter().sum::<f32>() / waveform.peaks.len() as f32;

            Some(PeakStats {
                min: peak_min,
                max: peak_max,
                average: peak_avg,
            })
        } else {
            None
        };

        Some(WaveformOutput {
            peaks_per_second: waveform.peaks_per_second,
            total_peaks: waveform.peaks.len(),
            channels: waveform.channels,
            peak_statistics: peak_stats,
        })
    } else {
        None
    };

    // Prepare spectral data if requested
    let spectral_data = if show_spectrum {
        let fingerprint = libflo_audio::extract_spectral_fingerprint(
            &samples,
            file_info.channels,
            file_info.sample_rate,
            None,
            None,
        );

        let hash_hex = format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            fingerprint.hash[0],
            fingerprint.hash[1],
            fingerprint.hash[2],
            fingerprint.hash[3],
            fingerprint.hash[4],
            fingerprint.hash[5],
            fingerprint.hash[6],
            fingerprint.hash[7]
        );

        Some(SpectralOutput {
            duration_ms: fingerprint.duration_ms,
            sample_rate: fingerprint.sample_rate,
            channels: fingerprint.channels,
            peak_frequency_bands: fingerprint.frequency_peaks.to_vec(),
            energy_profile: fingerprint.energy_profile.to_vec(),
            average_loudness: fingerprint.avg_loudness,
            spectral_hash_hex: hash_hex,
        })
    } else {
        None
    };

    if output_json {
        let output = AnalysisOutput {
            file_info: FileInfo {
                sample_rate: file_info.sample_rate,
                channels: file_info.channels,
                bit_depth: file_info.bit_depth,
                duration_secs: file_info.duration_secs,
                total_samples: file_info.total_samples,
            },
            loudness: LoudnessOutput {
                integrated_lufs: loudness.integrated_lufs,
                loudness_range_lu: loudness.loudness_range_lu,
                true_peak_dbtp: loudness.true_peak_dbtp,
                sample_peak_dbfs: loudness.sample_peak_dbfs,
            },
            waveform: waveform_data,
            spectral: spectral_data,
        };

        let json_str = serde_json::to_string_pretty(&output)
            .context("Failed to serialize analysis to JSON")?;
        println!("{}", json_str);
    } else {
        println!("Analyzing {}...", input.display());
        println!();

        println!("File Information");
        println!("────────────────");
        println!("  Sample rate: {} Hz", file_info.sample_rate);
        println!("  Channels:    {}", file_info.channels);
        println!("  Bit depth:   {} bits", file_info.bit_depth);
        println!("  Duration:    {:.2}s", file_info.duration_secs);
        println!("  Total samples: {}", file_info.total_samples);
        println!();

        println!("Loudness Metrics (EBU R128)");
        println!("────────────────────────────");
        println!(
            "  Integrated loudness: {:.2} LUFS",
            loudness.integrated_lufs
        );
        println!(
            "  Loudness range:      {:.2} LU",
            loudness.loudness_range_lu
        );
        println!("  True peak:           {:.2} dBTP", loudness.true_peak_dbtp);
        println!(
            "  Sample peak:         {:.2} dBFS",
            loudness.sample_peak_dbfs
        );
        println!();

        // Waveform peaks if requested
        if let Some(wf) = waveform_data {
            println!("Waveform Analysis");
            println!("─────────────────");
            println!("  Peaks per second:    {}", wf.peaks_per_second);
            println!("  Total peaks:         {}", wf.total_peaks);
            println!("  Channels:            {}", wf.channels);

            if let Some(stats) = wf.peak_statistics {
                println!("  Peak statistics:");
                println!("    Min:               {:.6}", stats.min);
                println!("    Max:               {:.6}", stats.max);
                println!("    Average:           {:.6}", stats.average);
            }
            println!();
        }

        // Spectral analysis if requested
        if let Some(sp) = spectral_data {
            println!("Spectral Analysis");
            println!("─────────────────");
            println!("  Duration:            {} ms", sp.duration_ms);
            println!("  Sample rate:         {} Hz", sp.sample_rate);
            println!("  Channels:            {}", sp.channels);
            println!("  Peak frequency bands: {:?}", sp.peak_frequency_bands);
            println!("  Energy profile (16 bands): {:?}", sp.energy_profile);
            println!("  Average loudness:    {}", sp.average_loudness);
            println!(
                "  Spectral hash (first 8 bytes):   {}",
                sp.spectral_hash_hex
            );
            println!();
        }
    }

    Ok(())
}

fn validate(input: &PathBuf) -> Result<()> {
    let flo_data = fs::read(input).context("Failed to read flo™ file")?;

    let is_valid =
        reflo::validate_flo(&flo_data).map_err(|_| anyhow::anyhow!("Validation failed"))?;

    if is_valid {
        println!("✓ {} is a valid flo™ file", input.display());
        Ok(())
    } else {
        bail!("✗ {} is not a valid flo™ file", input.display())
    }
}

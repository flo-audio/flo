use anyhow::{Context, Result};
use std::io::{Cursor, Write};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value};
use symphonia::core::probe::Hint;

/// Metadata extracted from audio file
#[derive(Debug, Default)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub composer: Option<String>,
    pub comment: Option<String>,
    pub bpm: Option<f32>,
    // Cover art stored as (mime_type, data)
    pub cover_art: Option<(String, Vec<u8>)>,
    // Source format (e.g., "MP3", "FLAC", "WAV")
    pub source_format: Option<String>,
    // Original filename
    pub original_filename: Option<String>,
}

/// Read an audio file and return (samples, sample_rate, channels, metadata)
/// Samples are interleaved f32 in range [-1.0, 1.0]
pub fn read_audio_file_with_metadata(path: &Path) -> Result<(Vec<f32>, u32, usize, AudioMetadata)> {
    let file = std::fs::File::open(path).context("Failed to open audio file")?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    read_from_source_with_metadata(mss, path.extension().and_then(|e| e.to_str()))
}

/// Read audio from bytes (for cross-platform/WASM support)
pub fn read_audio_from_bytes(bytes: &[u8]) -> Result<(Vec<f32>, u32, usize, AudioMetadata)> {
    let cursor = Cursor::new(bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    read_from_source_with_metadata(mss, None)
}

/// Read an audio file and return (samples, sample_rate, channels)
/// Samples are interleaved f32 in range [-1.0, 1.0]
#[allow(dead_code)]
pub fn read_audio_file(path: &Path) -> Result<(Vec<f32>, u32, usize)> {
    let (samples, sample_rate, channels, _) = read_audio_file_with_metadata(path)?;
    Ok((samples, sample_rate, channels))
}

fn read_from_source_with_metadata(
    mss: MediaSourceStream,
    extension: Option<&str>,
) -> Result<(Vec<f32>, u32, usize, AudioMetadata)> {
    // Create hint from file extension
    let mut hint = Hint::new();
    if let Some(ext) = extension {
        hint.with_extension(ext);
    }

    // Enable metadata reading
    let meta_opts = MetadataOptions {
        limit_metadata_bytes: symphonia::core::meta::Limit::Maximum(16 * 1024 * 1024), // 16MB max
        limit_visual_bytes: symphonia::core::meta::Limit::Maximum(16 * 1024 * 1024),
    };

    // Probe the format
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &meta_opts)
        .context("Unsupported audio format")?;

    let mut format = probed.format;

    // Extract metadata
    let mut metadata = AudioMetadata {
        source_format: extension.map(|ext| ext.to_uppercase()),
        ..Default::default()
    };

    // Check metadata from probe result
    if let Some(meta_rev) = probed.metadata.get() {
        if let Some(current) = meta_rev.current() {
            extract_metadata_tags(current, &mut metadata);
        }
    }

    // Also check format metadata
    if let Some(meta_rev) = format.metadata().current() {
        extract_metadata_tags(meta_rev, &mut metadata);
    }

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found")?;

    // If we didn't get format from extension, try to detect from codec
    if metadata.source_format.is_none() {
        let codec_type = track.codec_params.codec;
        metadata.source_format = Some(match codec_type {
            symphonia::core::codecs::CODEC_TYPE_FLAC => "FLAC".to_string(),
            symphonia::core::codecs::CODEC_TYPE_PCM_S16LE
            | symphonia::core::codecs::CODEC_TYPE_PCM_S16BE
            | symphonia::core::codecs::CODEC_TYPE_PCM_S24LE
            | symphonia::core::codecs::CODEC_TYPE_PCM_S32LE => "WAV".to_string(),
            symphonia::core::codecs::CODEC_TYPE_MP3 => "MP3".to_string(),
            symphonia::core::codecs::CODEC_TYPE_VORBIS => "OGG".to_string(),
            symphonia::core::codecs::CODEC_TYPE_AAC => "AAC".to_string(),
            _ => "UNKNOWN".to_string(),
        });
    }

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("Unknown sample rate")?;
    let channels = track
        .codec_params
        .channels
        .context("Unknown channel count")?
        .count();

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    let mut samples = Vec::new();

    // Decode all packets
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(e).context("Error reading packet"),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e).context("Error decoding packet"),
        };

        // Convert to f32
        append_samples(&decoded, &mut samples, channels);
    }

    Ok((samples, sample_rate, channels, metadata))
}

fn extract_metadata_tags(
    meta: &symphonia::core::meta::MetadataRevision,
    metadata: &mut AudioMetadata,
) {
    for tag in meta.tags() {
        if let Some(std_key) = tag.std_key {
            let value_str = match &tag.value {
                Value::String(s) => Some(s.clone()),
                _ => None,
            };

            match std_key {
                StandardTagKey::TrackTitle => metadata.title = value_str,
                StandardTagKey::Artist => metadata.artist = value_str,
                StandardTagKey::Album => metadata.album = value_str,
                StandardTagKey::AlbumArtist => metadata.album_artist = value_str,
                StandardTagKey::Date | StandardTagKey::ReleaseDate => {
                    if let Some(s) = value_str {
                        // Try to parse year from date string
                        if let Ok(year) = s.chars().take(4).collect::<String>().parse::<i32>() {
                            metadata.year = Some(year);
                        }
                    }
                }
                StandardTagKey::Genre => metadata.genre = value_str,
                StandardTagKey::TrackNumber => {
                    if let Value::UnsignedInt(n) = tag.value {
                        metadata.track_number = Some(n as u32);
                    } else if let Some(s) = value_str {
                        // Handle "1/12" format
                        if let Some(num) = s.split('/').next().and_then(|n| n.parse().ok()) {
                            metadata.track_number = Some(num);
                        }
                    }
                }
                StandardTagKey::TrackTotal => {
                    if let Value::UnsignedInt(n) = tag.value {
                        metadata.track_total = Some(n as u32);
                    }
                }
                StandardTagKey::DiscNumber => {
                    if let Value::UnsignedInt(n) = tag.value {
                        metadata.disc_number = Some(n as u32);
                    }
                }
                StandardTagKey::Composer => metadata.composer = value_str,
                StandardTagKey::Comment => metadata.comment = value_str,
                StandardTagKey::Bpm => {
                    if let Value::UnsignedInt(n) = tag.value {
                        metadata.bpm = Some(n as f32);
                    } else if let Some(s) = value_str {
                        metadata.bpm = s.parse().ok();
                    }
                }
                _ => {}
            }
        }
    }

    // Extract cover art from visuals
    for visual in meta.visuals() {
        if visual.usage == Some(symphonia::core::meta::StandardVisualKey::FrontCover)
            || metadata.cover_art.is_none()
        {
            let mime = visual.media_type.clone();
            metadata.cover_art = Some((mime, visual.data.to_vec()));
        }
    }
}

fn append_samples(buffer: &AudioBufferRef, samples: &mut Vec<f32>, channels: usize) {
    match buffer {
        AudioBufferRef::F32(buf) => {
            for frame in 0..buf.frames() {
                for ch in 0..channels {
                    samples.push(buf.chan(ch)[frame]);
                }
            }
        }
        AudioBufferRef::S16(buf) => {
            let scale = 1.0 / 32768.0;
            for frame in 0..buf.frames() {
                for ch in 0..channels {
                    samples.push(buf.chan(ch)[frame] as f32 * scale);
                }
            }
        }
        AudioBufferRef::S32(buf) => {
            let scale = 1.0 / 2147483648.0;
            for frame in 0..buf.frames() {
                for ch in 0..channels {
                    samples.push(buf.chan(ch)[frame] as f32 * scale);
                }
            }
        }
        AudioBufferRef::U8(buf) => {
            for frame in 0..buf.frames() {
                for ch in 0..channels {
                    samples.push((buf.chan(ch)[frame] as f32 - 128.0) / 128.0);
                }
            }
        }
        _ => {
            // For other formats, try to get f32 representation
            // This is a fallback
        }
    }
}

/// Write samples to a WAV file using symphonia
pub fn write_wav(path: &Path, samples: &[f32], sample_rate: u32, channels: usize) -> Result<()> {
    let bytes = write_wav_to_bytes(samples, sample_rate, channels)?;
    std::fs::write(path, bytes).context("Failed to write WAV file")
}

/// Write samples to WAV format in memory (for cross-platform/WASM support)
pub fn write_wav_to_bytes(samples: &[f32], sample_rate: u32, channels: usize) -> Result<Vec<u8>> {
    // WAV file format (RIFF)
    let mut buffer = Vec::new();

    let num_samples = samples.len();
    let bytes_per_sample = 4; // 32-bit float
    let data_size = num_samples * bytes_per_sample;
    let file_size = 36 + data_size; // 44 byte header - 8 + data_size

    // RIFF header
    buffer.write_all(b"RIFF")?;
    buffer.write_all(&(file_size as u32).to_le_bytes())?;
    buffer.write_all(b"WAVE")?;

    // fmt chunk
    buffer.write_all(b"fmt ")?;
    buffer.write_all(&16u32.to_le_bytes())?; // chunk size
    buffer.write_all(&3u16.to_le_bytes())?; // format = IEEE float
    buffer.write_all(&(channels as u16).to_le_bytes())?;
    buffer.write_all(&sample_rate.to_le_bytes())?;
    let byte_rate = sample_rate * channels as u32 * bytes_per_sample as u32;
    buffer.write_all(&byte_rate.to_le_bytes())?;
    let block_align = channels as u16 * bytes_per_sample as u16;
    buffer.write_all(&block_align.to_le_bytes())?;
    buffer.write_all(&32u16.to_le_bytes())?; // bits per sample

    // data chunk
    buffer.write_all(b"data")?;
    buffer.write_all(&(data_size as u32).to_le_bytes())?;

    // Write samples
    for &sample in samples {
        buffer.write_all(&sample.to_le_bytes())?;
    }

    Ok(buffer)
}

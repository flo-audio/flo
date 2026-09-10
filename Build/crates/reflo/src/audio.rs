use anyhow::{Context, Result};
use std::io::{Cursor, Write};
use std::path::Path;
use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{well_known, AudioDecoderOptions, CODEC_ID_NULL_AUDIO};
use symphonia::core::common::Limit;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTag, StandardVisualKey};

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
    let meta_opts = MetadataOptions::default()
        .limit_tag_bytes(Limit::Maximum(16 * 1024 * 1024)) // 16MB max
        .limit_visual_bytes(Limit::Maximum(16 * 1024 * 1024));

    // Probe the format
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), meta_opts)
        .context("Unsupported audio format")?;

    // Extract metadata
    let mut metadata = AudioMetadata {
        source_format: extension.map(|ext| ext.to_uppercase()),
        ..Default::default()
    };

    // Check metadata from the format reader
    if let Some(current) = format.metadata().current() {
        extract_metadata_tags(current, &mut metadata);
    }

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params
                .as_ref()
                .and_then(|p| p.audio())
                .is_some_and(|a| a.codec != CODEC_ID_NULL_AUDIO)
        })
        .context("No audio track found")?;

    let codec_params = track
        .codec_params
        .as_ref()
        .context("No codec parameters")?
        .audio()
        .context("No audio codec")?;

    // If we didn't get format from extension, try to detect from codec
    if metadata.source_format.is_none() {
        let codec_type = codec_params.codec;
        metadata.source_format = Some(match codec_type {
            well_known::CODEC_ID_FLAC => "FLAC".to_string(),
            well_known::CODEC_ID_PCM_S16LE
            | well_known::CODEC_ID_PCM_S16BE
            | well_known::CODEC_ID_PCM_S24LE
            | well_known::CODEC_ID_PCM_S32LE => "WAV".to_string(),
            well_known::CODEC_ID_MP3 => "MP3".to_string(),
            well_known::CODEC_ID_VORBIS => "OGG".to_string(),
            well_known::CODEC_ID_AAC => "AAC".to_string(),
            _ => "UNKNOWN".to_string(),
        });
    }

    let track_id = track.id;
    let sample_rate = codec_params.sample_rate.context("Unknown sample rate")?;
    let channels = codec_params
        .channels
        .clone()
        .context("Unknown channel count")?
        .count();

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(codec_params, &AudioDecoderOptions::default())
        .context("Failed to create decoder")?;

    let mut samples = Vec::new();

    // Decode all packets
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(e) => return Err(e).context("Error reading packet"),
        };

        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e).context("Error decoding packet"),
        };

        // Convert to f32
        append_samples(&decoded, &mut samples);
    }

    Ok((samples, sample_rate, channels, metadata))
}

fn extract_metadata_tags(
    meta: &symphonia::core::meta::MetadataRevision,
    metadata: &mut AudioMetadata,
) {
    for tag in &meta.media.tags {
        if let Some(std_tag) = &tag.std {
            match std_tag {
                StandardTag::TrackTitle(title) => metadata.title = Some(title.to_string()),
                StandardTag::Artist(artist) => metadata.artist = Some(artist.to_string()),
                StandardTag::Album(album) => metadata.album = Some(album.to_string()),
                StandardTag::AlbumArtist(artist) => {
                    metadata.album_artist = Some(artist.to_string())
                }
                StandardTag::RecordingYear(year) | StandardTag::ReleaseYear(year) => {
                    metadata.year = Some(*year as i32);
                }
                StandardTag::RecordingDate(date) | StandardTag::ReleaseDate(date) => {
                    if let Ok(year) = date.chars().take(4).collect::<String>().parse::<i32>() {
                        metadata.year = Some(year);
                    }
                }
                StandardTag::Genre(genre) => metadata.genre = Some(genre.to_string()),
                StandardTag::TrackNumber(n) => metadata.track_number = Some(*n as u32),
                StandardTag::TrackTotal(n) => metadata.track_total = Some(*n as u32),
                StandardTag::DiscNumber(n) => metadata.disc_number = Some(*n as u32),
                StandardTag::Composer(composer) => metadata.composer = Some(composer.to_string()),
                StandardTag::Comment(comment) => metadata.comment = Some(comment.to_string()),
                StandardTag::Bpm(bpm) => metadata.bpm = Some(*bpm as f32),
                _ => {}
            }
        }
    }

    // Extract cover art from visuals
    for visual in &meta.media.visuals {
        if matches!(visual.usage, Some(StandardVisualKey::FrontCover))
            || metadata.cover_art.is_none()
        {
            let mime = visual.media_type.clone().unwrap_or_default();
            metadata.cover_art = Some((mime, visual.data.to_vec()));
        }
    }
}

fn append_samples(decoded: &GenericAudioBufferRef, samples: &mut Vec<f32>) {
    let mut frame = Vec::new();
    decoded.copy_to_vec_interleaved(&mut frame);
    samples.extend_from_slice(&frame);
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

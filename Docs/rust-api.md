# Rust API

The `libflo` crate provides native Rust encoding, decoding, and metadata support.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
libflo = "0.1"
```

---

## Quick Start

```rust
use libflo::{Encoder, decode, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create 1 second of stereo silence
    let samples: Vec<f32> = vec![0.0; 44100 * 2];
    
    // Encode
    let encoder = Encoder::new(44100, 2, 16);
    let flo_data = encoder.encode(&samples, &[])?;
    
    // Decode
    let decoded = decode(&flo_data)?;
    
    // Info
    let file_info = info(&flo_data)?;
    println!("Duration: {}s", file_info.duration_secs);
    
    Ok(())
}
```

---

## Lossless Encoding

### Encoder

```rust
use libflo::Encoder;

let encoder = Encoder::new(
    sample_rate,  // u32: e.g., 44100
    channels,     // u8: 1 (mono) or 2 (stereo)
    bit_depth     // u8: 16, 24, or 32
);

// Encode without metadata
let flo_data = encoder.encode(&samples, &[])?;

// Encode with metadata
let metadata = create_metadata();
let flo_data = encoder.encode(&samples, &metadata)?;
```

### Sample Format

Samples must be:
- `Vec<f32>` or `&[f32]`
- Interleaved: `[L, R, L, R, ...]` for stereo
- Normalized: `-1.0` to `1.0`

```rust
// Mono: [s0, s1, s2, ...]
// Stereo: [L0, R0, L1, R1, L2, R2, ...]
```

---

## Lossy Encoding

### LossyEncoder

```rust
use libflo::{LossyEncoder, QualityPreset};

// With quality preset
let quality = QualityPreset::High.as_f32();
let mut encoder = LossyEncoder::new(44100, 2, quality);
let flo_data = encoder.encode_to_flo(&samples, &[])?;
```

### Quality Presets

```rust
use libflo::QualityPreset;

QualityPreset::Low         // 0.1  - ~48 kbps
QualityPreset::Medium      // 0.3  - ~128 kbps
QualityPreset::High        // 0.55 - ~192 kbps
QualityPreset::VeryHigh    // 0.75 - ~256 kbps
QualityPreset::Transparent // 0.95 - ~320 kbps

// Get f32 value
let quality: f32 = QualityPreset::High.as_f32();
```

### Target Bitrate

```rust
use libflo::{LossyEncoder, QualityPreset};

// Calculate quality from target bitrate
let quality = QualityPreset::from_bitrate(
    192,    // target kbps
    44100,  // sample rate
    2       // channels
).as_f32();

let mut encoder = LossyEncoder::new(44100, 2, quality);
let flo_data = encoder.encode_to_flo(&samples, &[])?;
```

---

## Decoding

### Auto-detect Mode

```rust
use libflo::decode;

// Automatically detects lossless vs lossy
let samples: Vec<f32> = decode(&flo_data)?;
```

### Decoder Struct

```rust
use libflo::Decoder;

let decoder = Decoder::new();
let samples = decoder.decode(&flo_data)?;
```

---

## File Information

```rust
use libflo::info;

let file_info = info(&flo_data)?;

println!("Sample rate: {}", file_info.sample_rate);
println!("Channels: {}", file_info.channels);
println!("Bit depth: {}", file_info.bit_depth);
println!("Duration: {}s", file_info.duration_secs);
println!("Lossy: {}", file_info.is_lossy);
println!("Compression: {}x", file_info.compression_ratio);
```

### AudioInfo Struct

```rust
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub total_samples: u64,
    pub duration_secs: f64,
    pub is_lossy: bool,
    pub compression_ratio: f32,
}
```

---

## Validation

```rust
use libflo::validate;

match validate(&flo_data) {
    Ok(true) => println!("File is valid"),
    Ok(false) => println!("CRC32 mismatch - file corrupted"),
    Err(e) => println!("Invalid file: {}", e),
}
```

---

## Metadata

### FloMetadata Struct

```rust
use libflo::{FloMetadata, SectionType, PictureType};

let mut meta = FloMetadata::new();

// Basic fields
meta.title = Some("Track Title".to_string());
meta.artist = Some("Artist Name".to_string());
meta.album = Some("Album Name".to_string());
meta.year = Some(2026);
meta.genre = Some("Electronic".to_string());
meta.bpm = Some(128);

// Track numbering
meta.track_number = Some(1);
meta.track_total = Some(12);
meta.disc_number = Some(1);
meta.disc_total = Some(2);

// Serialize to MessagePack
let metadata_bytes = meta.to_msgpack()?;

// Use in encoding
let encoder = Encoder::new(44100, 2, 16);
let flo_data = encoder.encode(&samples, &metadata_bytes)?;
```

### Section Markers

```rust
use libflo::{FloMetadata, SectionType};

let mut meta = FloMetadata::new();

meta.add_section(0, SectionType::Intro, None);
meta.add_section(15000, SectionType::Verse, Some("Verse 1"));
meta.add_section(45000, SectionType::Chorus, None);
meta.add_section(75000, SectionType::Verse, Some("Verse 2"));
meta.add_section(105000, SectionType::Chorus, None);
meta.add_section(135000, SectionType::Bridge, None);
meta.add_section(165000, SectionType::Outro, None);
```

### Section Types

```rust
pub enum SectionType {
    Intro, Verse, PreChorus, Chorus, PostChorus,
    Bridge, Breakdown, Drop, Buildup, Solo,
    Instrumental, Outro, Silence, Other
}
```

### Cover Art

```rust
use libflo::{FloMetadata, Picture, PictureType};
use std::fs;

let mut meta = FloMetadata::new();

let image_data = fs::read("cover.jpg")?;
meta.pictures = Some(vec![
    Picture {
        mime_type: "image/jpeg".to_string(),
        picture_type: PictureType::CoverFront,
        description: Some("Album Cover".to_string()),
        data: image_data,
    }
]);
```

### Synchronized Lyrics

```rust
use libflo::{FloMetadata, SyncedLyrics, LyricLine, LyricContentType};

let mut meta = FloMetadata::new();

meta.synced_lyrics = Some(vec![
    SyncedLyrics {
        language: Some("eng".to_string()),
        content_type: LyricContentType::Lyrics,
        description: None,
        lines: vec![
            LyricLine { timestamp_ms: 0, text: "First line of lyrics".to_string() },
            LyricLine { timestamp_ms: 5000, text: "Second line".to_string() },
            LyricLine { timestamp_ms: 10000, text: "Third line".to_string() },
        ],
    }
]);
```

### Reading Metadata

```rust
use libflo::get_metadata;

let meta = get_metadata(&flo_data)?;

if let Some(title) = meta.title {
    println!("Title: {}", title);
}

if let Some(sections) = meta.section_markers {
    for section in sections {
        println!("{:?} at {}ms", section.section_type, section.timestamp_ms);
    }
}
```

---

## Low-Level API

### Reader

```rust
use libflo::Reader;

let reader = Reader::new();
let flo_file = reader.read(&data)?;

// Access header
println!("Version: {}.{}", flo_file.header.version_major, flo_file.header.version_minor);
println!("Sample rate: {}", flo_file.header.sample_rate);

// Access frames
for frame in &flo_file.frames {
    println!("Frame type: {}, samples: {}", frame.frame_type, frame.frame_samples);
}
```

### Writer

```rust
use libflo::{Writer, FloFile};

let writer = Writer::new();
let data = writer.write(&flo_file)?;
```

---

## Error Handling

All functions return `FloResult<T>`:

```rust
use libflo::{decode, FloError};

match decode(&data) {
    Ok(samples) => println!("Decoded {} samples", samples.len()),
    Err(FloError::InvalidMagic) => println!("Not a flo™ file"),
    Err(FloError::UnsupportedVersion(v)) => println!("Version {} not supported", v),
    Err(FloError::CrcMismatch) => println!("File is corrupted"),
    Err(e) => println!("Error: {:?}", e),
}
```

### Error Types

```rust
pub enum FloError {
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidFrameType(u8),
    CrcMismatch,
    InsufficientData,
    EncodingError(String),
    DecodingError(String),
    MetadataError(String),
    IoError(std::io::Error),
}
```

---

## Feature Flags

```toml
[dependencies]
libflo = { version = "0.1", features = ["..."] }
```

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | ✓ | Standard library support |
| `wasm` | ✗ | WebAssembly bindings |

---

## Thread Safety

- `Encoder` and `Decoder` are `Send + Sync`
- Safe for concurrent encoding/decoding
- No global state

# flo™

A modern audio codec supporting both **lossless** and **lossy** compression.

- **Lossless mode**: Adaptive Linear Predictive Coding (ALPC) with Rice encoding (~2-3x compression)
- **Lossy mode**: MDCT-based psychoacoustic transform coding (~10-30x compression)

## Features

- **Dual-mode compression**: Lossless for perfect quality, lossy for smaller files
- **Lossy quality levels**: Low (~48kbps), Medium (~128kbps), High (~192kbps), VeryHigh (~256kbps), Transparent (~320kbps)
- **Psychoacoustic masking**: Automatically discards inaudible frequencies for efficient lossy encoding
- **Lossless compression** with typical 2-3x compression ratios
- **Mono and stereo** audio support
- **Multiple sample rates** (44100, 48000, etc.)
- **WebAssembly support** for browser-based encoding/decoding
- **CLI converter** for converting MP3, WAV, FLAC, OGG to flo™
- **Streaming decoder** for real-time playback and progressive loading
- **Rich metadata support** (ID3v2.4 compatible + flo-unique extensions)

## Compression Comparison

| Mode     | Quality     | Typical Ratio | Equivalent Bitrate | Use Case          |
|----------|-------------|---------------|--------------------|-------------------|
| Lossy    | Low         | ~30x          | ~48 kbps           | Speech, podcasts  |
| Lossy    | Medium      | ~10x          | ~128 kbps          | General music     |
| Lossy    | High        | ~6x           | ~192 kbps          | Quality listening |
| Lossy    | VeryHigh    | ~4x           | ~256 kbps          | Near-transparent  |
| Lossy    | Transparent | ~3x           | ~320 kbps          | Archival          |
| Lossless |      -      | ~2-3x         |          -         | Perfect quality   |

## Quick Start

### Build

```bash
# Build everything (native, CLI, WASM)
./scripts/build.sh

# Build only CLI converter
./scripts/build.sh reflo

# Build only WASM for web
./scripts/build.sh wasm
```

### CLI Usage

```bash
# Convert MP3 to flo™ (lossless)
flo encode music.mp3 music.flo

# Convert with lossy mode (smaller file)
flo encode music.mp3 music.flo --lossy --quality high

# Lossy with bitrate target
flo encode music.mp3 music.flo --bitrate 192

# With metadata
flo encode music.mp3 music.flo --title "My Song" --artist "Artist" --album "Album"

# Convert flo™ back to WAV  
flo decode music.flo music.wav

# Show file info
flo info music.flo

# Show info with metadata
flo info music.flo --metadata

# View full metadata (human-readable)
flo metadata music.flo

# View metadata as JSON
flo metadata music.flo --json

# Validate file
flo validate music.flo
```

### Web Demo

```bash
# Start dev server
./scripts/serve.sh

# Open http://localhost:8080
```

The web demo supports:
- **Encoding mode selection**: Toggle between lossless and lossy
- **Quality slider**: Choose from 5 lossy quality levels
- Generating test signals (sine wave, stereo test, white noise)
- Recording from microphone
- Uploading audio files (MP3, WAV, FLAC, OGG, etc.)
- Displaying metadata, cover art, sections, and synced lyrics
- Downloading as .flo or .wav

## Metadata

flo™ supports comprehensive metadata in MessagePack format, compatible with ~80% of ID3v2.4 fields plus flo-unique extensions.

### Lightning-Fast Metadata Editing

Unlike other formats where editing metadata requires:
- Re-encoding the entire file (lossy formats)
- Complex block manipulation (FLAC)
- Hope you have enough padding (MP3)

FLO's design separates metadata from audio data, allowing instant updates. No re-encoding. No quality loss. No waiting.

### Standard Fields (ID3v2.4 Compatible)

| Category           | Fields                                                                |
|--------------------|-----------------------------------------------------------------------|
| **Identification** | title, subtitle, album, track_number/total, disc_number/total, isrc   |
| **People**         | artist, album_artist, composer, conductor, lyricist, remixer          |
| **Properties**     | genre, mood, bpm, key, language                                       |
| **Dates**          | year, recording_time, release_time                                    |
| **Media**          | pictures (APIC), comments (COMM), lyrics (USLT), synced_lyrics (SYLT) |

### flo-Unique Extensions

| Feature                   | Description                                           |
|---------------------------|-------------------------------------------------------|
| **waveform_data**         | Pre-computed waveform peaks for instant visualization |
| **section_markers**       | Intro/verse/chorus/bridge/outro timestamps            |
| **bpm_map**               | Tempo changes throughout the track                    |
| **key_changes**           | Musical key changes with timestamps                   |
| **loudness_profile**      | Frame-by-frame LUFS measurements                      |
| **synced_lyrics**         | First-party SYLT support with timestamps              |
| **creator_notes**         | Timestamped producer/artist commentary                |
| **collaboration_credits** | Detailed per-person contribution tracking             |
| **remix_chain**           | Track derivation/sample history                       |
| **animated_cover**        | GIF/WebP animated artwork                             |
| **cover_variants**        | Multiple cover versions (explicit, clean, etc.)       |

### JavaScript API

```javascript
import { encode, get_metadata, get_cover_art, get_synced_lyrics } from './libflo.js';

// Create metadata
const metadata = create_metadata_from_object({
    title: "My Song",
    artist: "Artist",
    bpm: 128,
    section_markers: [
        { timestamp_ms: 0, section_type: "intro" },
        { timestamp_ms: 30000, section_type: "verse" }
    ]
});

// Encode with metadata
const floData = encode(samples, 44100, 2, 16, metadata);

// Read metadata
const meta = get_metadata(floData);
const cover = get_cover_art(floData);
const lyrics = get_synced_lyrics(floData);
```

## File Format

The flo™ format uses:
- Magic: `FLO!` (0x464C4F21)
- 66-byte header with chunk offsets
- CRC32 for integrity

### Encoding Modes

**Lossless (ALPC)**:
- Frame types 1-12 indicate LPC prediction order
- Rice-coded residuals for entropy compression
- Perfect bit-for-bit reconstruction

**Lossy (Transform)**:
- Frame type 253 indicates MDCT-based encoding
- 2048-sample blocks with 50% overlap (Vorbis window)
- Psychoacoustic model identifies masked frequencies
- Sparse RLE encoding for quantized coefficients

See [flo_audio.ksy](flo_audio.ksy) for the complete Kaitai Struct specification.

## Project Structure

```
flo/
├── libflo/                  # Core Rust library (also builds to WASM)
│   └── src/
│       ├── core/            # Core utilities (CRC32, Rice coding, types, metadata)
│       ├── lossless/        # Lossless encoder/decoder (ALPC)
│       └── lossy/           # Lossy encoder/decoder (MDCT, psychoacoustic)
├── reflo/             # CLI converter tool
├── Demo/                    # Web demo with JS frontend
├── Examples/                # Example flo™ files
├── scripts/                 # Build and test scripts
├── .github/workflows/       # CI configuration
└── flo_audio.ksy            # Kaitai Struct specification
```

## API

### Rust

```rust
use libflo_audio::{Encoder, Decoder, LossyEncoder, QualityPreset, FloMetadata, SectionType};

// === Lossless Encoding ===
let encoder = Encoder::new(44100, 2, 16);
let flo_data = encoder.encode(&samples, &metadata)?;

// === Lossy Encoding ===
// With quality preset
let quality = QualityPreset::High.as_f32();
let mut lossy = LossyEncoder::new(44100, 2, quality);
let flo_data = lossy.encode_to_flo(&samples, &metadata)?;

// With bitrate target
let quality = QualityPreset::from_bitrate(192, 44100, 2).as_f32();
let mut lossy = LossyEncoder::new(44100, 2, quality);
let flo_data = lossy.encode_to_flo(&samples, &metadata)?;

// === Metadata ===
let mut meta = FloMetadata::new();
meta.title = Some("My Song".to_string());
meta.artist = Some("Artist".to_string());
meta.add_section(0, SectionType::Intro, None);
meta.add_section(30000, SectionType::Verse, Some("Verse 1"));

let metadata = meta.to_msgpack()?;

// Encode with metadata
let encoder = Encoder::new(44100, 2, 16);
let flo_data = encoder.encode(&samples, &metadata)?;

// Decode (auto-detects lossless vs lossy)
let decoder = Decoder::new();
let samples = decoder.decode(&flo_data)?;
```

### JavaScript (WASM)

```javascript
import init, { 
    encode, encode_lossy, encode_transform, encode_with_bitrate,
    decode, info, validate, get_metadata 
} from './pkg-libflo/libflo.js';

await init();

// Lossless encoding
const floData = encode(samples, 44100, 2, 16, metadata);

// Lossy encoding with quality level (0-4)
// 0=Low, 1=Medium, 2=High, 3=VeryHigh, 4=Transparent
const floDataLossy = encode_lossy(samples, 44100, 2, 16, 2, metadata);

// Lossy encoding with continuous quality (0.0-1.0)
const floDataTransform = encode_transform(samples, 44100, 2, 16, 0.55, metadata);

// Lossy encoding with target bitrate (kbps)
const floDataBitrate = encode_with_bitrate(samples, 44100, 2, 16, 192, metadata);

// Decode (auto-detects mode)
const decoded = decode(floData);

// Get file info
const fileInfo = info(floData);
console.log(fileInfo.is_lossy, fileInfo.compression_ratio);

// Get metadata
const meta = get_metadata(floData);
```

### Streaming Decoder (JavaScript)

For real-time playback and progressive loading:

```javascript
import init, { WasmStreamingDecoder } from './pkg-libflo/libflo.js';

await init();

// Create streaming decoder
const decoder = new WasmStreamingDecoder();

// Feed data incrementally (e.g., from fetch chunks)
decoder.feed(chunk1);
decoder.feed(chunk2);

// Get file info once header is parsed
const info = decoder.get_info();
console.log(`Sample rate: ${info.sample_rate}, Channels: ${info.channels}`);

// Decode frame-by-frame for streaming playback
while (true) {
    const samples = decoder.next_frame();
    if (samples === null) break;
    
    // Schedule samples for audio playback
    playAudioSamples(samples);
}

// Or decode all available data at once
const allSamples = decoder.decode_available();

// Clean up
decoder.free();
```

## Supported Input Formats (CLI)

- WAV (via hound)
- MP3 (via symphonia)
- FLAC (via symphonia)
- OGG Vorbis (via symphonia)
- AAC/M4A (via symphonia)

## License

Copyright 2026 NellowTCS

The flo™ codec is licensed under the Apache License, Version 2.0. Check [LICENSE](LICENSE) for more details.

## Trademark Notice

"flo" and related branding are trademarks of NellowTCS. While the flo™ codec is open-source, the name "flo" and the branding are protected. You may not distribute modified versions of this software under the name "flo" without permission.

# libflo

A Rust library for encoding and decoding flo™ audio files with WASM support.

## Features

- **Dual-mode compression**: Lossless (ALPC) and lossy (MDCT transform)
- **ALPC Compression**: Adaptive Linear Predictive Coding (orders 1-12) for lossless
- **Transform Coding**: MDCT with psychoacoustic model for lossy (~10-30x compression)
- **Rice Coding**: Efficient residual compression
- **CRC32 Integrity**: Data chunk verification
- **Multi-channel**: Mono and stereo support
- **Streaming Decoder**: Frame-by-frame decoding for real-time playback
- **WASM Ready**: Full WebAssembly compatibility
- **Clean API**: Unified interface for CLI, WASM, and library use

## Module Structure

```
src/
├── lib.rs              # Main exports and WASM bindings
├── core/               # Core utilities
│   ├── crc32.rs        # CRC32 checksums
│   ├── rice.rs         # Rice coding for entropy
│   ├── types.rs        # Common types (Header, Frame, etc.)
│   └── metadata.rs     # Metadata structures (ID3-like)
├── lossless/           # Lossless mode (ALPC)
│   ├── encoder.rs      # Lossless encoder
│   ├── decoder.rs      # Lossless decoder
│   └── lpc.rs          # LPC analysis/synthesis
├── lossy/              # Lossy mode (Transform)
│   ├── encoder.rs      # Transform encoder (TransformEncoder)
│   ├── decoder.rs      # Transform decoder (TransformDecoder)
│   ├── mdct.rs         # MDCT/IMDCT transform
│   └── psychoacoustic.rs # Perceptual model
├── streaming/          # Streaming decoder
│   ├── encoder.rs      # Frame-by-frame streaming encoder
│   ├── decoder.rs      # Frame-by-frame streaming decoder
├── reader.rs           # Binary file parser
└── writer.rs           # Binary file writer
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
libflo = { path = "path/to/libflo" }
```

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `encode(samples, sample_rate, channels, bit_depth, metadata)` | Encode audio (lossless) |
| `encode_lossy(samples, sample_rate, channels, bit_depth, quality, metadata)` | Encode audio (lossy, quality 0-4) |
| `encode_transform(samples, sample_rate, channels, bit_depth, quality, metadata)` | Encode audio (lossy, quality 0.0-1.0) |
| `encode_with_bitrate(samples, sample_rate, channels, bit_depth, bitrate_kbps, metadata)` | Encode audio (lossy, target bitrate) |
| `decode(data)` | Decode flo™ file (auto-detects mode) |
| `validate(data)` | Verify file integrity (CRC32) |
| `info(data)` | Get file information |
| `version()` | Get library version |

### Metadata Functions (No Re-encode!)

flo™ stores metadata separately from audio data, enabling **instant** metadata updates without re-encoding.

| Function | Description |
|----------|-------------|
| `update_metadata(data, new_metadata)` | Update metadata without re-encoding (WASM) |
| `update_metadata_bytes(data, new_metadata)` | Update metadata without re-encoding (Rust) |
| `strip_metadata(data)` | Remove all metadata (WASM) |
| `strip_metadata_bytes(data)` | Remove all metadata (Rust) |
| `get_metadata_bytes(data)` | Get raw metadata bytes (WASM) |
| `get_metadata_bytes_native(data)` | Get raw metadata bytes (Rust) |
| `has_metadata(data)` | Check if file has metadata (fast header check) |

### Streaming Functions

| Function | Description |
|----------|-------------|
| `WasmStreamingDecoder::new()` | Create new streaming decoder |
| `feed(data)` | Feed bytes incrementally |
| `get_info()` | Get file info (sample rate, channels, etc.) |
| `next_frame()` | Decode next frame (returns samples or null) |
| `decode_available()` | Decode all buffered data at once |
| `reset()` | Reset decoder state |
| `free()` | Release resources |

### Structs

| Struct | Description |
|--------|-------------|
| `Encoder` | Lossless encoder instance |
| `LossyEncoder` | Transform-based lossy encoder |
| `Decoder` | Lossless decoder instance |
| `LossyDecoder` | Transform-based decoder |
| `QualityPreset` | Quality levels (Low, Medium, High, VeryHigh, Transparent) |
| `Reader` | Low-level binary parser |
| `Writer` | Low-level binary writer |
| `AudioInfo` | File information container |

## Quick Start

### Rust (Lossless)

```rust
use libflo::{Encoder, decode, info};

// Encode audio (lossless)
let samples: Vec<f32> = vec![0.0; 44100]; // 1 second of silence
let encoder = Encoder::new(44100, 2, 16);
let flo_data = encoder.encode(&samples, &[])?;

// Decode audio (auto-detects mode)
let decoded = decode(&flo_data)?;

// Get info
let file_info = info(&flo_data)?;
println!("Duration: {} seconds", file_info.duration_secs);
```

### Rust (Lossy)

```rust
use libflo::{LossyEncoder, QualityPreset, decode};

// Encode with quality preset
let quality = QualityPreset::High.as_f32(); // 0.55
let mut encoder = LossyEncoder::new(44100, 2, quality);
let flo_data = encoder.encode_to_flo(&samples, &[])?;

// Encode with bitrate target
let quality = QualityPreset::from_bitrate(192, 44100, 2).as_f32();
let mut encoder = LossyEncoder::new(44100, 2, quality);
let flo_data = encoder.encode_to_flo(&samples, &[])?;

// Decode (auto-detects lossy mode)
let decoded = decode(&flo_data)?;
```

### JavaScript (WASM)

```javascript
import init, { encode, encode_lossy, decode, info, validate, version } from './libflo.js';

await init();

// Lossless encode
const samples = new Float32Array(44100);
const floData = encode(samples, 44100, 1, 16, null);

// Lossy encode (quality: 0=Low, 1=Medium, 2=High, 3=VeryHigh, 4=Transparent)
const floDataLossy = encode_lossy(samples, 44100, 1, 16, 2, null);

// Decode (auto-detects mode)
const decoded = decode(floData);

// Validate
const isValid = validate(floData);

// Info
const fileInfo = info(floData);
console.log(`Lossy: ${fileInfo.is_lossy}, Ratio: ${fileInfo.compression_ratio}`);
```

### Streaming Decoder (JavaScript)

For real-time playback and progressive loading:

```javascript
import init, { WasmStreamingDecoder } from './libflo.js';

await init();

// Create streaming decoder
const decoder = new WasmStreamingDecoder();

// Feed data incrementally
decoder.feed(chunk);

// Get file info once header is parsed  
const info = decoder.get_info();

// Decode frame-by-frame
while (true) {
    const samples = decoder.next_frame();
    if (samples === null) break;
    // Play samples...
}

decoder.free();
```

## File Format

flo™ follows the specification in `flo_audio.ksy`:

```
┌─────────────────────────────────────┐
│ MAGIC "flo™!" (4 bytes)             │
├─────────────────────────────────────┤
│ HEADER (66 bytes)                   │
│   - version, sample_rate, channels  │
│   - bit_depth, compression_level    │
│   - CRC32, chunk sizes              │
├─────────────────────────────────────┤
│ TOC CHUNK                           │
│   - Frame seek table                │
│   - 20 bytes per entry              │
├─────────────────────────────────────┤
│ DATA CHUNK                          │
│   - Compressed audio frames         │
│   - 1 second per frame              │
├─────────────────────────────────────┤
│ EXTRA CHUNK (reserved)              │
├─────────────────────────────────────┤
│ META CHUNK (MessagePack)            │
└─────────────────────────────────────┘
```

### Frame Types

| Value | Type | Description |
|-------|------|-------------|
| 0 | Silence | No audio data stored |
| 1-12 | ALPC | Lossless LPC with order N |
| 253 | Transform | MDCT-based lossy encoding |
| 254 | Raw | Uncompressed PCM |
| 255 | Reserved | Future use |

## Building

### Native

```bash
cargo build --release
```

### WASM

```bash
cargo build --target wasm32-unknown-unknown --release
# Or with wasm-pack:
wasm-pack build --target web
```

## Testing

```bash
cargo test
```

## License

Apache-2.0

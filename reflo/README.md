# reflo

Audio format converter for the flo™ audio format.

## Features

- Convert audio files to/from flo™ format
- Support for multiple input formats (MP3, WAV, FLAC, OGG, AAC, etc.)
- Both lossless and lossy compression modes
- Metadata preservation (tags, cover art, lyrics, etc.)
- Cross-platform: Native (CLI) and WebAssembly (browser)
- Pure Rust implementation using Symphonia

## Installation

### As a CLI tool (Native)

```bash
cargo install --path . --features cli
```

Or build from source:

```bash
cargo build --release --features cli
```

The binary will be available at `target/release/flo`.

### As a library

Add to your `Cargo.toml`:

```toml
[dependencies]
reflo = { path = "path/to/reflo" }
```

### For WebAssembly

Install wasm-pack:

```bash
cargo install wasm-pack
```

Build for web:

```bash
wasm-pack build --target web --features wasm --no-default-features
```

This generates a `pkg/` directory with:
- `reflo.js` - JavaScript bindings
- `reflo_bg.wasm` - WebAssembly binary
- TypeScript definitions

## Usage

### CLI

```bash
# Encode to flo (lossless)
flo encode input.mp3 output.flo

# Encode with lossy compression
flo encode input.wav output.flo --lossy --quality high

# Encode with target bitrate
flo encode input.flac output.flo --lossy --bitrate 320

# Decode to WAV
flo decode input.flo output.wav

# Show file info
flo info input.flo

# Show metadata
flo metadata input.flo

# Validate file
flo validate input.flo
```

### Rust Library

```rust
use reflo::{encode_from_audio, decode_to_wav, EncodeOptions};
use std::fs;

// Read audio file
let audio_bytes = fs::read("input.mp3")?;

// Encode with options
let options = EncodeOptions::lossy(0.6) // High quality
    .with_level(5);

let flo_bytes = encode_from_audio(&audio_bytes, options)?;
fs::write("output.flo", flo_bytes)?;

// Decode
let flo_bytes = fs::read("output.flo")?;
let wav_bytes = decode_to_wav(&flo_bytes)?;
fs::write("output.wav", wav_bytes)?;
```

### WebAssembly

```html
<!DOCTYPE html>
<html>
<head>
    <script type="module">
        import init, { 
            encode_audio_to_flo, 
            decode_flo_to_wav,
            get_flo_file_info 
        } from './pkg-reflo/reflo.js';

        async function convertAudio() {
            await init();
            
            // Get file from input
            const file = document.getElementById('input').files[0];
            const audioBytes = new Uint8Array(await file.arrayBuffer());
            
            // Encode to flo (lossy, high quality)
            const floBytes = encode_audio_to_flo(
                audioBytes,
                true,  // lossy
                0.6,   // quality (0.0-1.0)
                5      // compression level
            );
            
            // Get info
            const info = get_flo_file_info(floBytes);
            console.log('Sample rate:', info.sample_rate);
            console.log('Channels:', info.channels);
            console.log('Compression:', info.compression_ratio);
            
            // Decode back to WAV
            const wavBytes = decode_flo_to_wav(floBytes);
            
            // Download
            const blob = new Blob([wavBytes], { type: 'audio/wav' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'output.wav';
            a.click();
        }
    </script>
</head>
<body>
    <input type="file" id="input" accept="audio/*">
    <button onclick="convertAudio()">Convert</button>
</body>
</html>
```

## API Reference

### Library Functions

- `encode_from_audio(bytes, options)` - Encode audio to flo format
- `encode_from_samples(samples, sr, ch, metadata, options)` - Encode raw samples
- `decode_to_wav(bytes)` - Decode flo to WAV format
- `decode_to_samples(bytes)` - Decode to raw samples
- `get_metadata(bytes)` - Extract metadata from flo file
- `get_flo_info(bytes)` - Get file information
- `get_audio_info(bytes)` - Get audio file information
- `validate_flo(bytes)` - Validate flo file integrity

### WebAssembly Functions

- `encode_audio_to_flo(audio_bytes, lossy, quality, level)` - Encode to flo
- `decode_flo_to_wav(flo_bytes)` - Decode to WAV
- `decode_flo_to_samples(flo_bytes)` - Decode to raw samples (returns object)
- `get_flo_file_info(flo_bytes)` - Get file info
- `get_audio_file_info(audio_bytes)` - Get audio info
- `get_flo_metadata_json(flo_bytes)` - Get metadata as JSON
- `validate_flo_file(flo_bytes)` - Validate file

## Architecture

The crate is structured for maximum code reuse:

```
reflo/
├── src/
│   ├── lib.rs      # Core library API (cross-platform)
│   ├── audio.rs    # Audio I/O using Symphonia
│   ├── main.rs     # CLI binary (feature: cli)
│   └── wasm.rs     # WASM bindings (feature: wasm)
```

### Cross-Platform Design

- **Core logic** in `lib.rs` works with `Vec<u8>` for portability
- **Audio I/O** in `audio.rs` uses Symphonia (works on all targets)
- **CLI interface** in `main.rs` adds file system operations
- **WASM interface** in `wasm.rs` adds JavaScript bindings

No platform-specific code in the core - it all works on native and WASM!

## Building

```bash
# Native CLI
cargo build --release --features cli

# Library only
cargo build --release --no-default-features

# WebAssembly
wasm-pack build --target web --features wasm --no-default-features

# All features (for testing)
cargo build --all-features
```

## License

Apache 2.0 License - see [LICENSE](LICENSE)

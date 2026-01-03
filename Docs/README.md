# flo™ Documentation

Welcome to the flo™ audio codec documentation!

## Quick Links

| Document | Description |
|----------|-------------|
| [Getting Started](getting-started.md) | Installation and first steps |
| [CLI Reference](cli-reference.md) | Complete command-line usage |
| [JavaScript API](javascript-api.md) | WASM API for browsers |
| [Rust API](rust-api.md) | Native Rust library usage |
| [File Format](file-format.md) | Technical specification |
| [Metadata Guide](metadata-guide.md) | Working with audio metadata |
| [Streaming](streaming.md) | Real-time streaming decoder |
| [Performance](performance.md) | Optimization tips |

## What is flo™?

flo™ (Fast Layered Object) is a modern audio codec supporting both **lossless** and **lossy** compression:

- **Lossless mode**: Perfect bit-for-bit reconstruction (~2-3x compression)
- **Lossy mode**: Psychoacoustic compression (~10-30x compression)

## Key Features

- **Dual-mode**: Choose lossless or lossy per-file
- **WebAssembly**: Full browser support
- **Rich metadata**: ID3v2.4 compatible + unique extensions
- **Streaming**: Frame-by-frame decoding for real-time playback
- **CLI tool**: Convert MP3, WAV, FLAC, OGG to flo™

## Components

### libflo
The core Rust library. Handles encoding, decoding, and metadata.
- Available as Rust crate and WASM module
- Pure Rust (uses `rustfft` for transforms)

### reflo
The command-line converter tool.
- Converts common formats to flo™
- Also available as WASM for browser-based conversion

## License

Apache-2.0. See [LICENSE](../LICENSE) for details.

"flo" is a trademark of NellowTCS.

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Dependencies:** bumped `symphonia` 0.5 -> 0.6.1 and migrated the reflo decode/metadata pipelines to the new API (`AudioCodecParameters`,
  `AudioDecoderOptions`, metadata revisions, probe-by-value options). All Rust workspace deps updated to their latest patch/minor versions (`serde`, `serde_json`, `serde_bytes`, `rmp-serde`, `serde-wasm-bindgen`, `rustfft`, `blake3`, `wasm-bindgen`, `js-sys`, `web-sys`, `anyhow`, `chrono`, `clap`).
- **Test restructure:** native Rust integration tests now live in `Tests/` (`Tests/libflo/`, `Tests/reflo/`) and are wired via `[[test]]` paths in the crate manifests; Jest WASM tests moved to `Tests/libflo/js/`.
- **Fixtures:** added `Build/crates/flo-fixtures`, an in-code signal synthesizer that replaces `sox`-based generation. `just examples` now writes `.wav`/`.flo` files to `Tests/fixtures/` and regenerates `Examples/`.

### Added

- `.devcontainer/` dev container (Rust + Node 24 + Python 3.14, wasm32 target, `just`, `cargo-deny`, `wasm-pack`).
- Toolchain version pins: `.nvmrc` (Node 24), `.python-version` (3.14), and an`.envrc` for direnv users.
- Governance documents: `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, and this changelog.

### Fixed

- Wasm encoder: `next_frame` returns `null` (not `undefined`) on end-of-stream; `flush()` now runs the final partial frame through `flush_into_frames`.
- ALPC `serialize_channel` serialization layout (order, coeffs, shift bits, residual encoding, Rice parameter) so encoded frames round-trip correctly.

## [0.1.2] - YYYY-MM-DD

Alpha release. Dual-mode lossless (ALPC + Rice) and lossy (MDCT psychoacoustic)
codec for WebAssembly and native, with metadata (ID3v2.4 compatible +
flo extensions), streaming encode/decode, and seeking support.

[Unreleased]: https://github.com/Audiflo/flo/compare/main...HEAD
[0.1.2]: https://github.com/Audiflo/flo/releases/tag/v0.1.2

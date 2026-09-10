# Contributing to flo

Thanks for contributing! This document covers how to set up a dev environment,
run checks, and what the project conventions are.

## Repository layout

```text
Build/
  Cargo.toml        Rust workspace root (profiles are configured here)
  crates/
    libflo/         The encoder/decoder codec library (Rust + WASM)
    reflo/          Converter CLI + library (uses symphonia for audio I/O)
    flo-fixtures/   In-code signal synthesis for Examples/ and Tests/fixtures/
Demo/               Browser demo (WASM bindings)
Docs/               Markdown documentation
Examples/           Generated .flo example files (see `just examples`)
Tests/
  libflo/           Rust integration tests + Jest (WASM) tests
  reflo/            reflo integration tests
  fixtures/         Canonical generated fixtures (see `just examples`)
scripts/            Python driver scripts (used by the Justfile)
Justfile            All common commands
```

## Prerequisites

- Rust (stable) with the `wasm32-unknown-unknown` target
- Node 24 (`.nvmrc`) — the Jest suite requires `NODE_OPTIONS=--experimental-vm-modules`
- Python 3.14 (`.python-version`)
- `just`, `cargo-deny`, `wasm-pack`

The easiest path is to open the repo in a Dev Container (`.devcontainer/`),
which installs all of the above. Otherwise:

```bash
cd scripts
python3 libflo.py setup; python3 reflo.py setup
```

## Commands

Everything lives behind `just`:

| Command                 | What it does                                      |
|-------------------------|---------------------------------------------------|
| `just check`            | Format check + lint + test + build for all crates |
| `just deny`             | cargo-deny license/advisory checks                |
| `just audit`            | cargo-audit vulnerability scan                    |
| `just examples`         | Regenerate `Examples/*.flo` and `Tests/fixtures/` |
| `just libflo test_wasm` | Run the Jest WASM test suite                      |
| `just reflo build`      | Build the CLI                                     |
| `just wasm`             | Build WASM packages for the demo                  |

## Conventions

- Rust code is formatted with `rustfmt` and linted with `clippy -D warnings`.
- Changes must keep `just check` and `just deny` green.
- New fixture/example audio should go through `flo-fixtures` — do not hand-add
  generated binaries. (Note: generated `.flo` files embed a build-time timestamp
  in their metadata, so `just examples` is not bit-reproducible across runs.)
- WebAssembly surfaces: keep `initSync({ module })`-style usage in tests; the
  Jest tests live in `Tests/libflo/js/`.
- Follow the existing file layout — native Rust tests in `Tests/libflo/rust/`
  and `Tests/reflo/`, referenced from `Tests/libflo/mod.rs`.

## Development workflow

1. Fork the repository and create a branch (`git checkout -b feature/name`).
2. Make your change. Add or update tests in `Tests/`.
3. Run `just check` and `just deny` locally.
4. Commit with a concise message describing the change.
5. Open a Pull Request with a clear description of the change and any tradeoffs.

Before opening a PR, double-check:

```bash
just examples          # if fixtures/examples changed
just libflo test_wasm  # if WASM/JS surface changed
```

## Code of conduct

All participants must follow the [Code of Conduct](CODE_OF_CONDUCT.md).

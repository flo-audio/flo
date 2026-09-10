# Security Policy

## Supported Versions

Only the most recent release is actively supported with security fixes.

| Version  | Supported          |
|----------|--------------------|
| latest   | :white_check_mark: |
| < latest | :x:                |

## Reporting a Vulnerability

Please do **not** open a public issue for security vulnerabilities. Report privately
via GitHub's private vulnerability reporting:

1. Go to <https://github.com/Audiflo/flo/security/advisories/new>
2. Describe the vulnerability, the affected version, and (if known) a proof of concept.

You can expect an acknowledgement within 48 hours and a status update on the
investigation within one week. If the issue is confirmed, a security advisory and
patch release will be published.

After resolution, we ask that reporters avoid public disclosure until a patched
release is available.

## Scope

The following areas are in scope:

- The `flo` binary format (see `flo_audio.ksy`)
- `Build/crates/libflo-audio` and `Build/crates/reflo` parsing/decoding paths
- WASM bindings and the web demo
- CI and release tooling under `.github/workflows`

Note: `cargo audit` and `cargo-deny` runs are wired into CI via `just deny` /
`just audit` (see `.github/workflows/security.yml`).

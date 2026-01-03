# CLI Reference

The `reflo` command-line tool converts audio files to and from the flo™ format.

## Synopsis

```bash
reflo <COMMAND> [OPTIONS]
```

## Commands

| Command | Description |
|---------|-------------|
| `encode` | Convert audio to flo™ format |
| `decode` | Convert flo™ to WAV |
| `info` | Display file information |
| `metadata` | Show detailed metadata |
| `validate` | Verify file integrity |
| `help` | Show help information |

---

## encode

Convert audio files to flo™ format.

### Usage
```bash
reflo encode <INPUT> <OUTPUT> [OPTIONS]
```

### Arguments
| Argument | Description |
|----------|-------------|
| `INPUT` | Source audio file (MP3, WAV, FLAC, OGG, AAC) |
| `OUTPUT` | Destination .flo file |

### Options

#### Compression Mode
| Option | Description |
|--------|-------------|
| `--lossless` | Use lossless compression (default) |
| `--lossy` | Use lossy compression |

#### Quality (Lossy Mode)
| Option | Values | Description |
|--------|--------|-------------|
| `--quality` | `low`, `medium`, `high`, `veryhigh`, `transparent` | Quality preset |
| `--bitrate` | Number (kbps) | Target bitrate (e.g., `192`) |

#### Metadata
| Option | Description |
|--------|-------------|
| `--title <TEXT>` | Set track title |
| `--artist <TEXT>` | Set artist name |
| `--album <TEXT>` | Set album name |
| `--year <YEAR>` | Set release year |
| `--genre <TEXT>` | Set genre |
| `--track <N>` | Set track number |
| `--cover <FILE>` | Set cover art image |

### Examples

```bash
# Basic lossless encoding
reflo encode song.mp3 song.flo

# Lossy with high quality
reflo encode song.mp3 song.flo --lossy --quality high

# Lossy with specific bitrate
reflo encode song.mp3 song.flo --lossy --bitrate 192

# With metadata
reflo encode song.mp3 song.flo \
  --title "My Song" \
  --artist "Artist Name" \
  --album "Album Name" \
  --year 2026

# With cover art
reflo encode song.mp3 song.flo --cover artwork.jpg
```

---

## decode

Convert flo™ files back to WAV format.

### Usage
```bash
reflo decode <INPUT> <OUTPUT>
```

### Arguments
| Argument | Description |
|----------|-------------|
| `INPUT` | Source .flo file |
| `OUTPUT` | Destination .wav file |

### Examples

```bash
# Basic decode
reflo decode song.flo song.wav
```

---

## info

Display file information.

### Usage
```bash
reflo info <FILE> [OPTIONS]
```

### Options
| Option | Description |
|--------|-------------|
| `--metadata` | Also show metadata |
| `--json` | Output as JSON |

### Output Fields
- Format version
- Sample rate
- Channels (mono/stereo)
- Bit depth
- Duration
- Compression mode (lossless/lossy)
- Compression ratio
- File size

### Examples

```bash
# Basic info
reflo info song.flo

# With metadata
reflo info song.flo --metadata

# JSON output (for scripts)
reflo info song.flo --json
```

---

## metadata

Display detailed metadata.

### Usage
```bash
reflo metadata <FILE> [OPTIONS]
```

### Options
| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |

### Examples

```bash
# Human-readable
reflo metadata song.flo

# JSON format
reflo metadata song.flo --json
```

---

## validate

Verify file integrity using CRC32 checksums.

### Usage
```bash
reflo validate <FILE>
```

### Exit Codes
| Code | Meaning |
|------|---------|
| 0 | File is valid |
| 1 | File is corrupted or invalid |

### Examples

```bash
# Validate a file
reflo validate song.flo

# Use in scripts
if reflo validate song.flo; then
  echo "File OK"
else
  echo "File corrupted!"
fi
```

---

## Supported Input Formats

| Format | Extension | Notes |
|--------|-----------|-------|
| WAV | `.wav` | PCM audio |
| MP3 | `.mp3` | MPEG Layer 3 |
| FLAC | `.flac` | Free Lossless Audio Codec |
| OGG | `.ogg` | Ogg Vorbis |
| AAC | `.aac`, `.m4a` | Advanced Audio Coding |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FLO_COMPRESSION_LEVEL` | Default compression level (0-9) |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | File not found |
| 4 | Unsupported format |
| 5 | Encoding error |
| 6 | Decoding error |

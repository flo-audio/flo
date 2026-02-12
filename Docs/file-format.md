# File Format Specification

Technical specification for the flo™ audio format.

## Overview

flo™ (Fast Layered Object) is a chunked audio format supporting both lossless and lossy compression.

**Key features:**

- Magic number identification
- CRC32 integrity verification
- Fixed-duration frames (1 second default)
- Chunk-based structure for seeking
- MessagePack metadata

---

## File Structure

```
┌─────────────────────────────────────┐
│ HEADER (66 bytes)                   │
│   Magic, version, audio params      │
│   CRC32, chunk sizes                │
├─────────────────────────────────────┤
│ TOC CHUNK                           │
│   Frame seek table                  │
├─────────────────────────────────────┤
│ DATA CHUNK                          │
│   Compressed audio frames           │
├─────────────────────────────────────┤
│ EXTRA CHUNK (reserved)              │
├─────────────────────────────────────┤
│ META CHUNK                          │
│   MessagePack metadata              │
└─────────────────────────────────────┘
```

---

## Header

66 bytes, little-endian:

| Offset | Size | Field               | Description                                           |
| ------ | ---- | ------------------- | ----------------------------------------------------- |
| 0      | 4    | `magic`             | `FLO!` (0x464C4F21)                                   |
| 4      | 1    | `version_major`     | Format version (1)                                    |
| 5      | 1    | `version_minor`     | Minor version (1)                                     |
| 6      | 2    | `flags`             | Bit flags (see below)                                 |
| 8      | 4    | `sample_rate`       | Hz (44100, 48000, etc.)                               |
| 12     | 1    | `channels`          | 1=mono, 2=stereo                                      |
| 13     | 1    | `bit_depth`         | 16, 24, or 32                                         |
| 14     | 8    | `total_frames`      | Duration in seconds (number of 1-second audio frames) |
| 22     | 1    | `compression_level` | Hint (0-9)                                            |
| 23     | 3    | `reserved`          | Must be 0                                             |
| 26     | 4    | `data_crc32`        | CRC32 of DATA chunk                                   |
| 30     | 8    | `header_size`       | Size of header (66)                                   |
| 38     | 8    | `toc_size`          | Size of TOC chunk                                     |
| 46     | 8    | `data_size`         | Size of DATA chunk                                    |
| 54     | 8    | `extra_size`        | Size of EXTRA chunk                                   |
| 62     | 8    | `meta_size`         | Size of META chunk                                    |

### Flags

| Bit   | Meaning                          |
| ----- | -------------------------------- |
| 0     | Lossy mode (0=lossless, 1=lossy) |
| 8-11  | Lossy quality level (0-4)        |
| Other | Reserved                         |

**Quality levels:** 0=Low, 1=Medium, 2=High, 3=VeryHigh, 4=Transparent

---

## TOC Chunk

Table of contents for seeking.

### Structure

| Offset | Size | Field         | Description           |
| ------ | ---- | ------------- | --------------------- |
| 0      | 4    | `num_entries` | Number of seek points |
| 4      | 20×N | `entries`     | Seek point array      |

### TOC Entry (20 bytes)

| Offset | Size | Field          | Description                  |
| ------ | ---- | -------------- | ---------------------------- |
| 0      | 4    | `frame_index`  | Frame number (0-based)       |
| 4      | 8    | `byte_offset`  | Offset from DATA chunk start |
| 12     | 4    | `frame_size`   | Size in bytes                |
| 16     | 4    | `timestamp_ms` | Time in milliseconds         |

---

## DATA Chunk

Contains compressed audio frames.

### Audio Frame

| Field           | Size     | Description               |
| --------------- | -------- | ------------------------- |
| `frame_type`    | 1        | Encoding type (see below) |
| `frame_samples` | 4        | Sample count              |
| `flags`         | 1        | Per-frame flags           |
| `channels`      | variable | Channel data array        |

### Frame Types

| Value | Name      | Description             |
| ----- | --------- | ----------------------- |
| 0     | Silence   | No data stored          |
| 1-12  | ALPC      | Lossless LPC order 1-12 |
| 253   | Transform | Lossy MDCT              |
| 254   | Raw       | Uncompressed PCM        |
| 255   | Reserved  | Future use              |

---

## Channel Data

Each channel is prefixed with its size:

```
┌──────────────────┬──────────────────┐
│ channel_size (4) │ channel_data     │
└──────────────────┴──────────────────┘
```

### ALPC Channel (Frame Types 1-12)

Adaptive Linear Predictive Coding for lossless compression.

| Field               | Size     | Description                  |
| ------------------- | -------- | ---------------------------- |
| `coeff_count`       | 1        | Number of LPC coefficients   |
| `predictor_coeffs`  | 4×N      | i32 coefficients             |
| `shift_bits`        | 1        | Dequantization shift         |
| `residual_encoding` | 1        | 0=Rice, 1=Golomb, 2=Raw      |
| `rice_parameter`    | 1        | Rice k value (if encoding=0) |
| `residuals`         | variable | Encoded residuals            |

**Reconstruction:**

```
sample[n] = residual[n] + Σ(coeff[i] × sample[n-1-i]) >> shift
```

### Transform Channel (Frame Type 253)

MDCT-based lossy compression.

| Field           | Size     | Description                                 |
| --------------- | -------- | ------------------------------------------- |
| `block_size`    | 1        | 0=Long(2048), 1=Short(256), 2=Start, 3=Stop |
| `scale_factors` | 50       | 25 bands × 2 bytes (log-scale u16)          |
| `coeff_length`  | 4        | Size of coefficient data                    |
| `coefficients`  | variable | Sparse RLE i16 values                       |

**Scale factor decode:**

```
scale = 2^((log_value - 32768) / 256)
```

**Coefficient decode:**

```
coeff[k] = quantized[k] / scale_factor[bark_band(k)]
```

### Sparse Coefficient Encoding

Coefficients use RLE for zero runs:

```
[zero_count (varint)] [non_zero_count (1)] [values (2×N)]
```

---

## EXTRA Chunk

Reserved for future extensions. Currently empty.

---

## META Chunk

MessagePack-encoded metadata.

### Standard Fields (ID3v2.4 compatible)

| Field          | Type   | ID3 Frame |
| -------------- | ------ | --------- |
| `title`        | string | TIT2      |
| `artist`       | string | TPE1      |
| `album`        | string | TALB      |
| `album_artist` | string | TPE2      |
| `composer`     | string | TCOM      |
| `genre`        | string | TCON      |
| `year`         | u32    | TYER      |
| `track_number` | u32    | TRCK      |
| `track_total`  | u32    | TRCK      |
| `disc_number`  | u32    | TPOS      |
| `disc_total`   | u32    | TPOS      |
| `bpm`          | u32    | TBPM      |
| `key`          | string | TKEY      |
| `isrc`         | string | TSRC      |
| `lyrics`       | string | USLT      |
| `comments`     | array  | COMM      |
| `pictures`     | array  | APIC      |

### flo™ Extensions

| Field              | Type   | Description                |
| ------------------ | ------ | -------------------------- |
| `section_markers`  | array  | Intro/verse/chorus markers |
| `bpm_map`          | array  | Tempo changes              |
| `key_changes`      | array  | Key signature changes      |
| `loudness_profile` | array  | LUFS per frame             |
| `waveform_data`    | object | Pre-computed peaks         |
| `synced_lyrics`    | array  | SYLT-style lyrics          |
| `creator_notes`    | array  | Producer commentary        |
| `animated_cover`   | object | GIF/WebP cover             |

### Section Marker

```javascript
{
  timestamp_ms: 30000,
  section_type: "chorus",
  label: "Chorus 1"  // optional
}
```

### Section Types

`intro`, `verse`, `pre_chorus`, `chorus`, `post_chorus`, `bridge`, `breakdown`, `drop`, `buildup`, `solo`, `instrumental`, `outro`, `silence`, `other`

### Picture

```javascript
{
  mime_type: "image/jpeg",
  picture_type: "cover_front",
  description: "Album art",
  data: Uint8Array
}
```

### Picture Types

`other`, `file_icon`, `cover_front`, `cover_back`, `leaflet_page`, `media`, `lead_artist`, `artist`, `conductor`, `band`, `composer`, `lyricist`, `recording_location`, `during_recording`, `during_performance`, `video_screen_capture`, `illustration`, `band_logo`, `publisher_logo`

---

## CRC32

The DATA chunk is verified using CRC32 (IEEE 802.3 polynomial).

**Verification:**

1. Read `data_crc32` from header
2. Compute CRC32 of entire DATA chunk
3. Compare values

---

## Byte Order

All multi-byte values are **little-endian**.

---

## Version History

| Version | Changes         |
| ------- | --------------- |
| 1.0     | Initial release |

---

## Kaitai Struct

A complete Kaitai Struct specification is available at [flo_audio.ksy](../flo_audio.ksy).

```bash
# Generate parser for your language
kaitai-struct-compiler -t python flo_audio.ksy
kaitai-struct-compiler -t javascript flo_audio.ksy
```

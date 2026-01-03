# Metadata Guide

flo™ supports rich metadata including ID3v2.4-compatible fields and unique extensions for modern music production.

## Quick Start

### CLI

```bash
# Add basic metadata during encoding
reflo encode song.mp3 song.flo \
  --title "My Song" \
  --artist "Artist Name" \
  --album "Album Name" \
  --year 2026 \
  --genre "Electronic" \
  --track 1 \
  --cover artwork.jpg
```

### JavaScript

```javascript
import { create_metadata_from_object, encode } from '@flo-audio/libflo';

const metadata = create_metadata_from_object({
  title: "My Song",
  artist: "Artist Name",
  album: "Album Name",
  year: 2026,
  genre: "Electronic",
  bpm: 128
});

const floData = encode(samples, 44100, 2, 16, metadata);
```

### Rust

```rust
use libflo_audio::{FloMetadata, Encoder};

let mut meta = FloMetadata::new();
meta.title = Some("My Song".to_string());
meta.artist = Some("Artist Name".to_string());
meta.album = Some("Album Name".to_string());
meta.year = Some(2026);

let metadata = meta.to_msgpack()?;
let encoder = Encoder::new(44100, 2, 16);
let flo_data = encoder.encode(&samples, &metadata)?;
```

---

## Standard Fields

### Identification

| Field | Type | Description | ID3 |
|-------|------|-------------|-----|
| `title` | string | Track title | TIT2 |
| `subtitle` | string | Subtitle/description | TIT3 |
| `album` | string | Album name | TALB |
| `isrc` | string | International Standard Recording Code | TSRC |

### Track Info

| Field | Type | Description | ID3 |
|-------|------|-------------|-----|
| `track_number` | number | Track position | TRCK |
| `track_total` | number | Total tracks | TRCK |
| `disc_number` | number | Disc number | TPOS |
| `disc_total` | number | Total discs | TPOS |

### People

| Field | Type | Description | ID3 |
|-------|------|-------------|-----|
| `artist` | string | Main artist | TPE1 |
| `album_artist` | string | Album artist | TPE2 |
| `composer` | string | Composer | TCOM |
| `conductor` | string | Conductor | TPE3 |
| `lyricist` | string | Lyricist | TEXT |
| `remixer` | string | Remix artist | TPE4 |

### Properties

| Field | Type | Description | ID3 |
|-------|------|-------------|-----|
| `genre` | string | Genre | TCON |
| `mood` | string | Mood (e.g., "energetic") | TMOO |
| `bpm` | number | Beats per minute | TBPM |
| `key` | string | Musical key (e.g., "Am", "C#") | TKEY |
| `language` | string | Language code (e.g., "eng") | TLAN |

### Dates

| Field | Type | Description | ID3 |
|-------|------|-------------|-----|
| `year` | number | Release year | TYER |
| `recording_time` | string | Recording date/time | TDRC |
| `release_time` | string | Release date/time | TDRL |

### Copyright

| Field | Type | Description | ID3 |
|-------|------|-------------|-----|
| `copyright` | string | Copyright notice | TCOP |
| `publisher` | string | Publisher name | TPUB |

---

## Cover Art

### Adding Cover Art (CLI)

```bash
reflo encode song.mp3 song.flo --cover cover.jpg
```

### Adding Cover Art (JavaScript)

```javascript
const coverData = await fetch('cover.jpg').then(r => r.arrayBuffer());

const metadata = create_metadata_from_object({
  title: "My Song",
  pictures: [{
    mime_type: "image/jpeg",
    picture_type: "cover_front",
    description: "Album Cover",
    data: new Uint8Array(coverData)
  }]
});
```

### Reading Cover Art

```javascript
import { get_cover_art } from '@flo-audio/libflo';

const cover = get_cover_art(floData);
if (cover) {
  const blob = new Blob([cover.data], { type: cover.mime_type });
  const url = URL.createObjectURL(blob);
  document.getElementById('cover-img').src = url;
}
```

### Picture Types

| Type | Description |
|------|-------------|
| `cover_front` | Front album cover (most common) |
| `cover_back` | Back album cover |
| `artist` | Artist photo |
| `band` | Band photo |
| `file_icon` | Small icon |
| `leaflet_page` | Booklet page |
| `media` | CD/vinyl image |

---

## flo™ Extensions

### Section Markers

Mark song structure for visualization and navigation.

```javascript
const metadata = create_metadata_from_object({
  title: "My Song",
  section_markers: [
    { timestamp_ms: 0, section_type: "intro" },
    { timestamp_ms: 15000, section_type: "verse", label: "Verse 1" },
    { timestamp_ms: 45000, section_type: "chorus" },
    { timestamp_ms: 75000, section_type: "verse", label: "Verse 2" },
    { timestamp_ms: 105000, section_type: "chorus" },
    { timestamp_ms: 135000, section_type: "bridge" },
    { timestamp_ms: 165000, section_type: "outro" }
  ]
});
```

**Section Types:**
- `intro`, `outro`
- `verse`, `pre_chorus`, `chorus`, `post_chorus`
- `bridge`, `breakdown`, `buildup`, `drop`
- `solo`, `instrumental`
- `silence`, `other`

### BPM Map

Track tempo changes throughout the song.

```javascript
const metadata = create_metadata_from_object({
  bpm: 120, // Starting BPM
  bpm_map: [
    { timestamp_ms: 0, bpm: 120 },
    { timestamp_ms: 60000, bpm: 130 },  // Speed up at 1:00
    { timestamp_ms: 120000, bpm: 120 }  // Back to normal
  ]
});
```

### Key Changes

Track key signature changes.

```javascript
const metadata = create_metadata_from_object({
  key: "Am", // Starting key
  key_changes: [
    { timestamp_ms: 0, key: "Am" },
    { timestamp_ms: 90000, key: "C" },   // Modulate to C major
    { timestamp_ms: 150000, key: "Am" }  // Return to A minor
  ]
});
```

### Synchronized Lyrics

Lyrics with timestamps for karaoke-style display.

```javascript
const metadata = create_metadata_from_object({
  synced_lyrics: [{
    language: "eng",
    content_type: "lyrics",
    lines: [
      { timestamp_ms: 5000, text: "First line of the song" },
      { timestamp_ms: 10000, text: "Second line here" },
      { timestamp_ms: 15000, text: "And the third line" }
    ]
  }]
});
```

**Reading synced lyrics:**

```javascript
import { get_synced_lyrics } from '@flo-audio/libflo';

const lyrics = get_synced_lyrics(floData);
if (lyrics) {
  for (const line of lyrics) {
    console.log(`${line.timestamp_ms}ms: ${line.text}`);
  }
}

// Highlight current line during playback
function getCurrentLine(lyrics, currentTimeMs) {
  for (let i = lyrics.length - 1; i >= 0; i--) {
    if (lyrics[i].timestamp_ms <= currentTimeMs) {
      return lyrics[i];
    }
  }
  return null;
}
```

### Waveform Data

Pre-computed waveform peaks for instant visualization.

```javascript
const metadata = create_metadata_from_object({
  waveform_data: {
    peaks_per_second: 100,
    channels: 2,
    peaks: [
      0.1, 0.2, 0.5, 0.8, 0.3, // ... peak values -1.0 to 1.0
    ]
  }
});
```

### Loudness Profile

Frame-by-frame LUFS measurements.

```javascript
const metadata = create_metadata_from_object({
  integrated_loudness_lufs: -14.0,
  loudness_range_lu: 8.5,
  true_peak_dbtp: -1.0,
  loudness_profile: [
    { timestamp_ms: 0, lufs: -18.0 },
    { timestamp_ms: 1000, lufs: -14.0 },
    { timestamp_ms: 2000, lufs: -12.0 }
  ]
});
```

### Creator Notes

Timestamped producer/artist commentary.

```javascript
const metadata = create_metadata_from_object({
  creator_notes: [
    { timestamp_ms: 0, text: "Recorded at Sunset Studios, LA" },
    { timestamp_ms: 45000, text: "This synth is a Moog Model D" },
    { timestamp_ms: 90000, text: "Guitar solo by John Smith" }
  ]
});
```

### Animated Cover

GIF or WebP animated artwork.

```javascript
const gifData = await fetch('cover.gif').then(r => r.arrayBuffer());

const metadata = create_metadata_from_object({
  animated_cover: {
    mime_type: "image/gif",
    data: new Uint8Array(gifData),
    duration_ms: 3000,
    loop_count: 0  // 0 = infinite
  }
});
```

---

## Reading Metadata

### JavaScript

```javascript
import { get_metadata, get_cover_art, get_synced_lyrics } from '@flo-audio/libflo';

// Get all metadata
const meta = get_metadata(floData);
console.log(meta.title, meta.artist, meta.album);

// Get cover art
const cover = get_cover_art(floData);

// Get synced lyrics
const lyrics = get_synced_lyrics(floData);
```

### Rust

```rust
use libflo_audio::get_metadata;

let meta = get_metadata(&flo_data)?;

if let Some(title) = &meta.title {
    println!("Title: {}", title);
}

if let Some(markers) = &meta.section_markers {
    for marker in markers {
        println!("{:?} at {}ms", marker.section_type, marker.timestamp_ms);
    }
}
```

### CLI

```bash
# Human-readable
reflo metadata song.flo

# JSON format
reflo metadata song.flo --json
```

---

## Best Practices

1. **Always include basic fields**: `title`, `artist`, `album`
2. **Use high-quality cover art**: JPEG or PNG, at least 500×500px
3. **Add section markers**: Improves user navigation
4. **Include BPM**: Helps DJs and music apps
5. **Use standard key notation**: "Am", "C#m", "F", etc.
6. **Keep synced lyrics accurate**: Test with actual playback

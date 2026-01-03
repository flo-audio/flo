# JavaScript API

The libflo WASM module provides full encoding, decoding, and metadata support for browsers.

## Installation

### npm
```bash
npm install @flo-audio/libflo
```

### CDN / Direct
```html
<script type="module">
  import init, * as libflo from './pkg-libflo/libflo_audio.js';
  await init();
</script>
```

---

## Initialization

Always call `init()` before using any functions:

```javascript
import init, { encode, decode } from '@flo-audio/libflo';

await init();
// Now you can use the API
```

---

## Core Functions

### encode()

Encode audio samples to lossless flo™ format.

```javascript
encode(samples, sampleRate, channels, bitDepth, metadata) → Uint8Array
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `samples` | `Float32Array` | Interleaved audio samples (-1.0 to 1.0) |
| `sampleRate` | `number` | Sample rate (e.g., 44100, 48000) |
| `channels` | `number` | Number of channels (1 or 2) |
| `bitDepth` | `number` | Bit depth (16, 24, or 32) |
| `metadata` | `Uint8Array \| null` | Optional MessagePack metadata |

**Returns:** `Uint8Array` - Encoded flo™ data

```javascript
const samples = new Float32Array(44100 * 2); // 1 sec stereo
const floData = encode(samples, 44100, 2, 16, null);
```

---

### encode_lossy()

Encode audio with lossy compression using quality presets.

```javascript
encode_lossy(samples, sampleRate, channels, bitDepth, quality, metadata) → Uint8Array
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `quality` | `number` | Quality level: 0=Low, 1=Medium, 2=High, 3=VeryHigh, 4=Transparent |

```javascript
// High quality lossy encoding
const floData = encode_lossy(samples, 44100, 2, 16, 2, null);
```

---

### encode_transform()

Encode with continuous quality control (0.0 to 1.0).

```javascript
encode_transform(samples, sampleRate, channels, bitDepth, quality, metadata) → Uint8Array
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `quality` | `number` | Quality from 0.0 (lowest) to 1.0 (highest) |

```javascript
// 55% quality (roughly "high")
const floData = encode_transform(samples, 44100, 2, 16, 0.55, null);
```

---

### encode_with_bitrate()

Encode targeting a specific bitrate.

```javascript
encode_with_bitrate(samples, sampleRate, channels, bitDepth, bitrateKbps, metadata) → Uint8Array
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `bitrateKbps` | `number` | Target bitrate in kbps (e.g., 128, 192, 320) |

```javascript
// Target 192 kbps
const floData = encode_with_bitrate(samples, 44100, 2, 16, 192, null);
```

---

### decode()

Decode flo™ data to audio samples. Auto-detects lossless vs lossy.

```javascript
decode(data) → Float32Array
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `data` | `Uint8Array` | flo™ file data |

**Returns:** `Float32Array` - Interleaved audio samples

```javascript
const samples = decode(floData);
// samples.length = totalSamples * channels
```

---

### info()

Get file information without decoding.

```javascript
info(data) → Object
```

**Returns:**
```javascript
{
  sample_rate: number,      // e.g., 44100
  channels: number,         // 1 or 2
  bit_depth: number,        // 16, 24, or 32
  total_samples: number,    // Total sample count
  duration_secs: number,    // Duration in seconds
  is_lossy: boolean,        // true if lossy mode
  compression_ratio: number // e.g., 2.5 for 2.5x compression
}
```

```javascript
const fileInfo = info(floData);
console.log(`${fileInfo.duration_secs}s, ${fileInfo.is_lossy ? 'lossy' : 'lossless'}`);
```

---

### validate()

Verify file integrity using CRC32.

```javascript
validate(data) → boolean
```

```javascript
if (validate(floData)) {
  console.log('File is valid');
} else {
  console.log('File is corrupted');
}
```

---

## Metadata Functions

### get_metadata()

Extract metadata as a JavaScript object.

```javascript
get_metadata(data) → Object | null
```

```javascript
const meta = get_metadata(floData);
if (meta) {
  console.log(meta.title, meta.artist);
}
```

---

### get_cover_art()

Extract cover art image.

```javascript
get_cover_art(data) → Object | null
```

**Returns:**
```javascript
{
  mime_type: string,  // e.g., "image/jpeg"
  data: Uint8Array    // Image bytes
}
```

```javascript
const cover = get_cover_art(floData);
if (cover) {
  const blob = new Blob([cover.data], { type: cover.mime_type });
  const url = URL.createObjectURL(blob);
  document.getElementById('cover').src = url;
}
```

---

### get_synced_lyrics()

Extract synchronized lyrics.

```javascript
get_synced_lyrics(data) → Array | null
```

**Returns:**
```javascript
[
  { timestamp_ms: 0, text: "First line..." },
  { timestamp_ms: 5000, text: "Second line..." },
  // ...
]
```

---

### create_metadata_from_object()

Create metadata bytes from a JavaScript object.

```javascript
create_metadata_from_object(obj) → Uint8Array
```

```javascript
const metadata = create_metadata_from_object({
  title: "My Song",
  artist: "Artist Name",
  album: "Album Name",
  year: 2026,
  genre: "Electronic",
  bpm: 128,
  section_markers: [
    { timestamp_ms: 0, section_type: "intro" },
    { timestamp_ms: 30000, section_type: "verse" },
    { timestamp_ms: 60000, section_type: "chorus" }
  ]
});

const floData = encode(samples, 44100, 2, 16, metadata);
```

---

## Streaming Decoder

For real-time playback and progressive loading. See [Streaming Guide](streaming.md) for details.

```javascript
import { WasmStreamingDecoder } from '@flo-audio/libflo';

const decoder = new WasmStreamingDecoder();

// Feed data incrementally
decoder.feed(chunk1);
decoder.feed(chunk2);

// Get info once header is parsed
const info = decoder.get_info();

// Decode frame-by-frame
while (true) {
  const samples = decoder.next_frame();
  if (samples === null) break;
  playAudio(samples);
}

decoder.free();
```

---

## Working with Web Audio API

### From AudioBuffer to flo™

```javascript
async function encodeAudioBuffer(audioBuffer) {
  const channels = audioBuffer.numberOfChannels;
  const length = audioBuffer.length;
  
  // Interleave channels
  const samples = new Float32Array(length * channels);
  for (let i = 0; i < length; i++) {
    for (let ch = 0; ch < channels; ch++) {
      samples[i * channels + ch] = audioBuffer.getChannelData(ch)[i];
    }
  }
  
  return encode(samples, audioBuffer.sampleRate, channels, 16, null);
}
```

### From flo™ to AudioBuffer

```javascript
async function decodeToAudioBuffer(floData, audioContext) {
  const samples = decode(floData);
  const fileInfo = info(floData);
  
  const audioBuffer = audioContext.createBuffer(
    fileInfo.channels,
    samples.length / fileInfo.channels,
    fileInfo.sample_rate
  );
  
  // Deinterleave channels
  for (let ch = 0; ch < fileInfo.channels; ch++) {
    const channelData = audioBuffer.getChannelData(ch);
    for (let i = 0; i < channelData.length; i++) {
      channelData[i] = samples[i * fileInfo.channels + ch];
    }
  }
  
  return audioBuffer;
}
```

---

## Error Handling

All functions throw on error:

```javascript
try {
  const samples = decode(floData);
} catch (error) {
  console.error('Decode failed:', error.message);
}
```

Common errors:
- `"Invalid magic bytes"` - Not a flo™ file
- `"Unsupported version"` - File version too new
- `"CRC32 mismatch"` - File is corrupted
- `"Invalid frame type"` - Malformed audio data

# flo™ Example Files

This directory contains example `.flo` audio files demonstrating different use cases and encoding options.

## Lossless Examples

| File | Description | Duration | Sample Rate | Channels | Size |
|------|-------------|----------|-------------|----------|------|
| `sine_440hz_mono.flo` | Pure 440Hz A4 sine wave | 2s | 44.1kHz | Mono | 56KB |
| `chord_cmajor_stereo.flo` | C major chord (C4+E4+G4) | 2s | 44.1kHz | Stereo | 79KB |
| `sweep_20_20k.flo` | Frequency sweep 20Hz→20kHz | 5s | 44.1kHz | Mono | 229KB |
| `white_noise.flo` | Random white noise | 1s | 44.1kHz | Mono | 81KB |
| `silence_1sec.flo` | Complete silence | 1s | 44.1kHz | Mono | **125B** |
| `click_track_120bpm.flo` | Metronome at 120 BPM | 4s | 44.1kHz | Mono | 133KB |
| `multitone_stereo.flo` | Different tones L/R | 2s | 44.1kHz | Stereo | 162KB |
| `dtmf_tones.flo` | Phone dial tones 0-9 | 3s | 44.1kHz | Mono | 138KB |
| `hires_96khz.flo` | Hi-res 1kHz tone | 1s | 96kHz | Mono | 63KB |
| `telephone_8khz.flo` | Telephone quality | 1s | 8kHz | Mono | 8KB |

## Lossy Examples

These demonstrate the quality/size tradeoffs at different quality settings:

| File | Quality | Size | Notes |
|------|---------|------|-------|
| `lossy_chord_low.flo` | 0.2 (Low) | 18KB | Highest compression, audible artifacts |
| `lossy_chord_medium.flo` | 0.4 (Medium) | 20KB | Good for speech/podcasts |
| `lossy_chord_high.flo` | 0.6 (High) | 22KB | Good for music |
| `lossy_chord_veryhigh.flo` | 0.8 (Very High) | 26KB | Near-transparent |
| `lossy_chord_transparent.flo` | 1.0 (Transparent) | 219KB | Perceptually lossless |
| `lossy_music_pattern.flo` | 0.6 (High) | 65KB | Synthesized music pattern |

## Test Patterns

### Silence Detection
`silence_1sec.flo` is only **125 bytes** for 1 second of silence, demonstrating flo™'s silence frame optimization.

### Compression Efficiency
Compare the same C major chord:
- **Lossless**: 79KB (`chord_cmajor_stereo.flo`)
- **Lossy High**: 22KB (`lossy_chord_high.flo`) - 72% smaller!
- **Lossy Low**: 18KB (`lossy_chord_low.flo`) - 77% smaller!

### Sample Rate Support
flo™ supports a wide range of sample rates:
- `telephone_8khz.flo` - 8kHz (telephone)
- Standard files - 44.1kHz (CD quality)
- `hires_96khz.flo` - 96kHz (hi-res audio)

## Playing Examples

### Using the Demo
Open the [web demo](../Demo/index.html) and drag any `.flo` file onto the page.

### Using reflo CLI
```bash
# Convert to WAV for playback
reflo decode sine_440hz_mono.flo -o sine.wav

# Get file info
reflo info sine_440hz_mono.flo
```

### Using JavaScript
```javascript
import init, { decode, info } from './libflo.js';

await init();
const floData = await fetch('sine_440hz_mono.flo').then(r => r.arrayBuffer());
const samples = decode(new Uint8Array(floData));
// Play samples with Web Audio API...
```

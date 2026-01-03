# Streaming Decoder

The streaming decoder enables real-time playback and progressive loading of flo™ files.

## Why Streaming?

| Scenario | Standard Decode | Streaming |
|----------|----------------|-----------|
| Large files (>10MB) |  Wait for full decode |  Play while loading |
| Network streaming |  Download entire file |  Play as chunks arrive |
| Memory usage |  Full file in memory |  Constant memory |
| Time to first sound |  Seconds |  Milliseconds |

---

## JavaScript API

### Basic Usage

```javascript
import init, { WasmStreamingDecoder } from '@flo-audio/libflo';

await init();

// Create decoder
const decoder = new WasmStreamingDecoder();

// Feed data (can be called multiple times)
decoder.feed(chunk);

// Get file info once header is parsed
const info = decoder.get_info();
if (info) {
  console.log(`${info.sample_rate}Hz, ${info.channels} channels`);
}

// Decode all available data at once
const samples = decoder.decode_available();

// Or decode frame-by-frame
while (true) {
  const frame = decoder.next_frame();
  if (frame === null) break;
  playAudio(frame);
}

// Clean up when done
decoder.free();
```

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `new WasmStreamingDecoder()` | `WasmStreamingDecoder` | Create new decoder |
| `feed(data)` | `void` | Feed bytes (Uint8Array) |
| `get_info()` | `Object \| null` | Get file info (null if header not yet parsed) |
| `decode_available()` | `Float32Array` | Decode all buffered data |
| `next_frame()` | `Float32Array \| null` | Get next frame (null if none available) |
| `available_frames()` | `number` | Number of frames ready to decode |
| `current_frame_index()` | `number` | Current position in file |
| `reset()` | `void` | Reset decoder state |
| `free()` | `void` | Release resources |

### Info Object

```javascript
{
  sample_rate: 44100,
  channels: 2,
  bit_depth: 16,
  total_frames: 180,        // Total frames in file
  is_lossy: false,
  lossy_quality: null       // 0-4 if lossy
}
```

---

## Streaming from Network

### Fetch with Streaming

```javascript
async function streamFromUrl(url) {
  const decoder = new WasmStreamingDecoder();
  const audioContext = new AudioContext();
  
  const response = await fetch(url);
  const reader = response.body.getReader();
  
  // Buffer for scheduling audio
  const sampleBuffer = [];
  let nextTime = audioContext.currentTime;
  
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    
    // Feed chunk to decoder
    decoder.feed(value);
    
    // Decode and schedule all available frames
    while (true) {
      const samples = decoder.next_frame();
      if (samples === null) break;
      
      // Schedule playback
      const info = decoder.get_info();
      const audioBuffer = samplesToAudioBuffer(samples, info, audioContext);
      
      const source = audioContext.createBufferSource();
      source.buffer = audioBuffer;
      source.connect(audioContext.destination);
      source.start(nextTime);
      
      nextTime += audioBuffer.duration;
    }
  }
  
  decoder.free();
}
```

### With Progress Callback

```javascript
async function streamWithProgress(url, onProgress) {
  const response = await fetch(url);
  const contentLength = parseInt(response.headers.get('Content-Length') || '0');
  const reader = response.body.getReader();
  
  const decoder = new WasmStreamingDecoder();
  let loaded = 0;
  
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    
    loaded += value.length;
    onProgress(loaded / contentLength);
    
    decoder.feed(value);
    // Process frames...
  }
  
  decoder.free();
}
```

---

## Real-Time Playback

### Using AudioWorklet

For glitch-free playback, use an AudioWorklet:

**audio-processor.js:**
```javascript
class FloProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.buffer = [];
    this.port.onmessage = (e) => {
      this.buffer.push(...e.data);
    };
  }
  
  process(inputs, outputs) {
    const output = outputs[0];
    const channel0 = output[0];
    const channel1 = output[1];
    
    for (let i = 0; i < channel0.length; i++) {
      if (this.buffer.length >= 2) {
        channel0[i] = this.buffer.shift();
        channel1[i] = this.buffer.shift();
      } else {
        channel0[i] = 0;
        channel1[i] = 0;
      }
    }
    
    return true;
  }
}

registerProcessor('flo-processor', FloProcessor);
```

**Main code:**
```javascript
const audioContext = new AudioContext();
await audioContext.audioWorklet.addModule('audio-processor.js');

const processor = new AudioWorkletNode(audioContext, 'flo-processor');
processor.connect(audioContext.destination);

// Feed samples to worklet
function sendSamplesToWorklet(samples) {
  processor.port.postMessage(Array.from(samples));
}

// Streaming loop
const decoder = new WasmStreamingDecoder();
decoder.feed(data);

while (true) {
  const samples = decoder.next_frame();
  if (samples === null) break;
  sendSamplesToWorklet(samples);
}
```

---

## Buffering Strategy

For smooth playback, buffer frames before starting:

```javascript
const MIN_BUFFER_SAMPLES = 8192; // ~185ms at 44100Hz

async function streamWithBuffering(url) {
  const decoder = new WasmStreamingDecoder();
  const audioContext = new AudioContext();
  
  let sampleBuffer = new Float32Array(0);
  let playing = false;
  let nextTime = 0;
  
  const response = await fetch(url);
  const reader = response.body.getReader();
  
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    
    decoder.feed(value);
    
    // Collect frames
    while (true) {
      const samples = decoder.next_frame();
      if (samples === null) break;
      
      // Append to buffer
      const newBuffer = new Float32Array(sampleBuffer.length + samples.length);
      newBuffer.set(sampleBuffer);
      newBuffer.set(samples, sampleBuffer.length);
      sampleBuffer = newBuffer;
    }
    
    // Start playback once buffer is full
    if (!playing && sampleBuffer.length >= MIN_BUFFER_SAMPLES) {
      playing = true;
      nextTime = audioContext.currentTime;
    }
    
    // Schedule buffered audio
    if (playing && sampleBuffer.length >= MIN_BUFFER_SAMPLES) {
      scheduleAudio(sampleBuffer, audioContext, nextTime);
      nextTime += sampleBuffer.length / 2 / 44100; // stereo
      sampleBuffer = new Float32Array(0);
    }
  }
  
  // Play remaining buffer
  if (sampleBuffer.length > 0) {
    scheduleAudio(sampleBuffer, audioContext, nextTime);
  }
  
  decoder.free();
}
```

---

## Frame Sizes

Frame sizes vary by encoding mode:

| Mode | Samples per Frame | Duration at 44.1kHz |
|------|-------------------|---------------------|
| Lossless | 44100 | 1 second |
| Lossy | 1024-2048 | ~23-46ms |

For lossy files, use buffering to collect multiple small frames before scheduling.

---

## Error Handling

```javascript
const decoder = new WasmStreamingDecoder();

try {
  decoder.feed(chunk);
  
  const info = decoder.get_info();
  if (!info) {
    console.log('Header not yet received');
    return;
  }
  
  const samples = decoder.next_frame();
  // ...
  
} catch (error) {
  console.error('Streaming error:', error.message);
} finally {
  decoder.free();
}
```

---

## Memory Management

Always call `free()` when done:

```javascript
const decoder = new WasmStreamingDecoder();

try {
  // Use decoder...
} finally {
  decoder.free(); // Release WASM memory
}
```

The decoder holds:
- Input buffer (grows as you feed data)
- Internal decode state
- Overlap buffers (for lossy MDCT)

Calling `reset()` clears decode state but keeps the decoder usable.
Calling `free()` releases all memory and invalidates the decoder.

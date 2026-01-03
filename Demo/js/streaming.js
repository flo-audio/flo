import { WasmStreamingDecoder } from '../pkg-libflo/libflo.js';
import { log } from './ui.js';

let streamingDemo = null;

// how far ahead to schedule audio chunks
const SCHEDULE_AHEAD_TIME = 0.3;
const MIN_BUFFER_SAMPLES = 8192;

// stream from a file with real frame-by-frame decoding
export async function streamFromFile(file) {
    if (streamingDemo?.active) {
        stopStreaming();
    }
    
    const audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    
    streamingDemo = {
        decoder: new WasmStreamingDecoder(),
        audioCtx,
        active: true,
        playing: false,
        totalBytesReceived: 0,
        framesDecoded: 0,
        startTime: performance.now(),
        info: null,
        nextPlayTime: 0,
        scheduledSources: [],
        fileSize: file.size,
        allReceived: false,
        // accumulate small frames before scheduling
        sampleBuffer: [],
        bufferedSamples: 0
    };
    
    log('Starting streaming playback...', 'info');
    updateStreamingUI('connecting');
    
    try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        
        // fake network chunks (4-16KB) for demo purposes
        let offset = 0;
        while (offset < bytes.length && streamingDemo?.active) {
            const chunkSize = Math.min(
                Math.floor(Math.random() * 12000) + 4000,
                bytes.length - offset
            );
            const chunk = bytes.slice(offset, offset + chunkSize);
            offset += chunkSize;
            
            await processChunk(chunk);
            
            // fake network latency
            await new Promise(r => setTimeout(r, Math.random() * 80 + 20));
        }
        
        // done receiving
        if (streamingDemo) {
            streamingDemo.allReceived = true;
            await processRemainingFrames();
            
            const elapsed = performance.now() - streamingDemo.startTime;
            log(`Stream complete: ${streamingDemo.framesDecoded} frames in ${elapsed.toFixed(0)}ms`, 'success');
        }
        
    } catch (err) {
        if (err.message && !err.message.includes('null')) {
            log(`Streaming failed: ${err.message}`, 'error');
            updateStreamingUI('error');
        }
        stopStreaming();
    }
}

// feed chunk to decoder and play any frames we get
async function processChunk(chunk) {
    if (!streamingDemo?.active) return;
    
    const { decoder, audioCtx } = streamingDemo;
    streamingDemo.totalBytesReceived += chunk.length;
    
    try {
        decoder.feed(chunk);
        
        // check if header parsed yet
        if (!streamingDemo.info && decoder.is_ready()) {
            streamingDemo.info = decoder.get_info();
            if (streamingDemo.info) {
                const { sample_rate, channels, is_lossy } = streamingDemo.info;
                log(`Stream info: ${sample_rate}Hz, ${channels}ch, ${is_lossy ? 'lossy' : 'lossless'}`);
                updateStreamingUI('buffering');
                
                streamingDemo.nextPlayTime = audioCtx.currentTime + SCHEDULE_AHEAD_TIME;
            }
        }
        
        // decode and play
        if (streamingDemo?.info && decoder.is_ready()) {
            await decodeAndScheduleFrames();
        }
        
        updateStreamingProgress();
        
    } catch (err) {
        // chunk errors are fine
    }
}

// decode frames and schedule them for playback, buffering small ones
async function decodeAndScheduleFrames() {
    if (!streamingDemo?.active || !streamingDemo.info) return;
    
    const { decoder, audioCtx, info } = streamingDemo;
    const channels = info.channels || 2;
    const sampleRate = info.sample_rate || 44100;
    
    // wake up the audio context
    if (audioCtx.state === 'suspended') {
        await audioCtx.resume();
    }
    
    // grab all available frames
    let frameCount = 0;
    let errorCount = 0;
    
    while (streamingDemo?.active) {
        try {
            const samples = decoder.next_frame();
            
            if (samples === null) {
                break;
            }
            
            if (!(samples instanceof Float32Array) || samples.length === 0) {
                continue;
            }
            
            frameCount++;
            streamingDemo.framesDecoded++;
            
            // buffer it up (hehe)
            streamingDemo.sampleBuffer.push(samples);
            streamingDemo.bufferedSamples += samples.length;
            
            // start playing after a few frames
            if (!streamingDemo.playing && streamingDemo.framesDecoded >= 3) {
                streamingDemo.playing = true;
                updateStreamingUI('playing');
                log('Playback started (streaming)', 'success');
            }
            
        } catch (err) {
            errorCount++;
            if (errorCount > 5) break;
        }
    }
    
    // flush buffer when we have enough or when done receiving
    if (streamingDemo?.active && streamingDemo.sampleBuffer.length > 0) {
        const shouldFlush = streamingDemo.bufferedSamples >= MIN_BUFFER_SAMPLES || 
                           streamingDemo.allReceived ||
                           streamingDemo.sampleBuffer.length >= 10;
        
        if (shouldFlush) {
            flushSampleBuffer(channels, sampleRate);
        }
    }
    
}


// push buffered samples to audio output
function flushSampleBuffer(channels, sampleRate) {
    if (!streamingDemo?.active || streamingDemo.sampleBuffer.length === 0) return;
    
    const { audioCtx, sampleBuffer } = streamingDemo;
    
    // smush all the frames together
    const totalSamples = streamingDemo.bufferedSamples;
    const combined = new Float32Array(totalSamples);
    let offset = 0;
    
    for (const frame of sampleBuffer) {
        combined.set(frame, offset);
        offset += frame.length;
    }
    
    streamingDemo.sampleBuffer = [];
    streamingDemo.bufferedSamples = 0;
    
    scheduleAudioPlayback(combined, channels, sampleRate);
}

// queue up samples for playback
function scheduleAudioPlayback(samples, channels, sampleRate) {
    if (!streamingDemo?.active) return;
    
    const { audioCtx } = streamingDemo;
    
    const samplesPerChannel = Math.floor(samples.length / channels);
    if (samplesPerChannel === 0) return;
    
    const duration = samplesPerChannel / sampleRate;
    
    const audioBuffer = audioCtx.createBuffer(channels, samplesPerChannel, sampleRate);
    
    // deinterleave
    for (let ch = 0; ch < channels; ch++) {
        const channelData = audioBuffer.getChannelData(ch);
        for (let i = 0; i < samplesPerChannel; i++) {
            channelData[i] = samples[i * channels + ch];
        }
    }
    
    const source = audioCtx.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(audioCtx.destination);
    
    // dont schedule in the past that would be weird
    const now = audioCtx.currentTime;
    if (streamingDemo.nextPlayTime < now) {
        // we fell behind, reset
        streamingDemo.nextPlayTime = now + 0.01;
    }
    
    source.start(streamingDemo.nextPlayTime);
    streamingDemo.nextPlayTime += duration;
    
    streamingDemo.scheduledSources.push(source);
    source.onended = () => {
        const idx = streamingDemo?.scheduledSources?.indexOf(source);
        if (idx !== undefined && idx >= 0) {
            streamingDemo?.scheduledSources?.splice(idx, 1);
        }
    };
}

// finish decoding after all data received
async function processRemainingFrames() {
    if (!streamingDemo?.active) return;
    
    const { info } = streamingDemo;
    if (!info) return;
    
    const channels = info.channels || 2;
    const sampleRate = info.sample_rate || 44100;
    
    // get the last frames
    await decodeAndScheduleFrames();
    
    // push out anything left
    if (streamingDemo?.sampleBuffer?.length > 0) {
        flushSampleBuffer(channels, sampleRate);
    }
    
    if (streamingDemo?.playing) {
        const remaining = streamingDemo.nextPlayTime - streamingDemo.audioCtx.currentTime;
        if (remaining > 0) {
            log(`Playback finishing in ${remaining.toFixed(1)}s...`);
            
            setTimeout(() => {
                if (streamingDemo && streamingDemo.scheduledSources?.length === 0) {
                    updateStreamingUI('ready');
                    streamingDemo.playing = false;
                    log('Playback complete', 'success');
                }
            }, remaining * 1000 + 100);
        }
    }
}

// resume playback
export function playStreamedAudio() {
    if (!streamingDemo) {
        log('No stream active', 'warning');
        return;
    }
    
    const { audioCtx } = streamingDemo;
    if (audioCtx.state === 'suspended') {
        audioCtx.resume().then(() => log('Audio resumed'));
    }
}

// stop everything
export function stopStreaming() {
    if (!streamingDemo) return;
    
    const demo = streamingDemo;
    streamingDemo = null; // null first to prevent further access
    
    demo.active = false;
    demo.playing = false;
    
    demo.scheduledSources?.forEach(s => {
        try { s.stop(); } catch(e) {}
    });
    
    if (demo.audioCtx) {
        try { demo.audioCtx.close(); } catch(e) {}
    }
    
    if (demo.decoder) {
        try { demo.decoder.free(); } catch(e) {}
    }
    
    updateStreamingUI('idle');
    log('Streaming stopped');
}

// update the streaming status ui
function updateStreamingUI(status) {
    const statusEl = document.getElementById('streamingStatus');
    const playBtn = document.getElementById('streamPlayBtn');
    const stopBtn = document.getElementById('streamStopBtn');
    
    if (statusEl) {
        const statusText = {
            idle: 'Ready',
            connecting: 'Connecting...',
            buffering: 'Buffering...',
            playing: 'Playing (streaming)',
            ready: 'Ready to stream',
            error: 'Error'
        };
        statusEl.textContent = statusText[status] || status;
        statusEl.className = `streaming-status status-${status}`;
    }
    
    if (playBtn) {
        playBtn.disabled = status !== 'ready';
    }
    
    if (stopBtn) {
        stopBtn.disabled = status === 'idle' || status === 'ready';
    }
}

// show how much we got so far
function updateStreamingProgress() {
    const progressEl = document.getElementById('streamingProgress');
    if (progressEl && streamingDemo) {
        const kb = (streamingDemo.totalBytesReceived / 1024).toFixed(1);
        const percent = streamingDemo.fileSize 
            ? Math.round((streamingDemo.totalBytesReceived / streamingDemo.fileSize) * 100)
            : 0;
        const frames = streamingDemo.framesDecoded;
        progressEl.textContent = `${kb} KB (${percent}%) • ${frames} frames`;
    }
}

// for debugging
export function getStreamingInfo() {
    if (!streamingDemo) return null;
    return {
        active: streamingDemo.active,
        playing: streamingDemo.playing,
        bytesReceived: streamingDemo.totalBytesReceived,
        framesDecoded: streamingDemo.framesDecoded,
        info: streamingDemo.info,
        bufferedSamples: streamingDemo.bufferedSamples,
        availableFrames: streamingDemo.decoder?.available_frames?.() ?? 0,
        currentFrameIndex: streamingDemo.decoder?.current_frame_index?.() ?? 0
    };
}

// debug helper: compare streaming vs standard decode
export async function debugCompareDecodeMethods(file) {
    const bytes = new Uint8Array(await file.arrayBuffer());
    console.log(`Testing file: ${file.name}, ${bytes.length} bytes`);
    
    // try decode_available()
    const decoder1 = new WasmStreamingDecoder();
    decoder1.feed(bytes);
    const info = decoder1.get_info();
    console.log('File info:', info);
    const available = decoder1.decode_available();
    console.log(`decode_available: ${available.length} samples`);
    
    // try next_frame() loop
    const decoder2 = new WasmStreamingDecoder();
    decoder2.feed(bytes);
    
    const frames = [];
    let frameCount = 0;
    while (true) {
        const samples = decoder2.next_frame();
        if (samples === null) break;
        if (samples.length > 0) {
            frames.push(samples);
            frameCount++;
        }
    }
    
    // smush frames together
    const totalLen = frames.reduce((sum, f) => sum + f.length, 0);
    const streamed = new Float32Array(totalLen);
    let offset = 0;
    for (const frame of frames) {
        streamed.set(frame, offset);
        offset += frame.length;
    }
    
    console.log(`next_frame: ${frameCount} frames, ${totalLen} samples`);
    
    // compare em
    if (available.length !== streamed.length) {
        console.error(`LENGTH MISMATCH: decode_available=${available.length}, next_frame=${streamed.length}`);
    }
    
    let maxDiff = 0;
    let diffCount = 0;
    const minLen = Math.min(available.length, streamed.length);
    for (let i = 0; i < minLen; i++) {
        const diff = Math.abs(available[i] - streamed[i]);
        if (diff > 0.0001) {
            if (diffCount < 10) {
                console.log(`Sample ${i}: available=${available[i]}, streamed=${streamed[i]}, diff=${diff}`);
            }
            diffCount++;
        }
        maxDiff = Math.max(maxDiff, diff);
    }
    
    console.log(`Max diff: ${maxDiff}, samples with diff: ${diffCount}`);
    
    // any bad values?
    const availableNaN = available.filter(s => !isFinite(s)).length;
    const streamedNaN = streamed.filter(s => !isFinite(s)).length;
    console.log(`NaN/Infinity: available=${availableNaN}, streamed=${streamedNaN}`);
    
    // whats the range look like
    const availableMin = Math.min(...available.slice(0, 10000));
    const availableMax = Math.max(...available.slice(0, 10000));
    const streamedMin = Math.min(...streamed.slice(0, 10000));
    const streamedMax = Math.max(...streamed.slice(0, 10000));
    console.log(`Range: available=[${availableMin.toFixed(4)}, ${availableMax.toFixed(4)}], streamed=[${streamedMin.toFixed(4)}, ${streamedMax.toFixed(4)}]`);
    
    decoder1.free();
    decoder2.free();
    
    return { available, streamed, maxDiff, diffCount };
}

// check if we can stream
export function isStreamingAvailable() {
    return typeof WasmStreamingDecoder !== 'undefined';
}

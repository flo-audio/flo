import { get_audio_file_info } from '../pkg-reflo/reflo.js';
import { state } from './state.js';
import { log } from './ui.js';
import { encodeAndUpdateUI } from './encoder.js';
import { stopAudio } from './playback.js';

// handle file input for any audio file or flo
export async function handleFile(file) {
    if (!file) return;
    
    stopAudio();
    
    log(`\nLoading ${file.name}...`);
    
    try {
        const arrayBuffer = await file.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);
        
        // is it a flo file?
        if (file.name.endsWith('.flo') || 
            (bytes[0] === 0x46 && bytes[1] === 0x4C && bytes[2] === 0x4F && bytes[3] === 0x21)) {
            // dynamic import to dodge circular deps
            const { decodeFloFile } = await import('./decoder.js');
            await decodeFloFile(bytes);
        } else {
            const audioInfo = get_audio_file_info(bytes);
            
            log(`  Format: ${file.name.split('.').pop().toUpperCase()}`);
            log(`  Sample rate: ${audioInfo.sampleRate}Hz`);
            log(`  Channels: ${audioInfo.channels}`);
            log(`  Duration: ${audioInfo.durationSecs.toFixed(1)}s`);
            
            // stash the raw bytes, reflo decodes when encoding
            state.audioFileBytes = bytes;
            state.sourceFileName = file.name;
            
            await encodeAndUpdateUI();
        }
        
    } catch (err) {
        log(`Failed to load file: ${err.message}`, 'error');
        console.error('File loading error:', err);
    }
}

// build a wav file from samples
function generateWavBytes(samples, sampleRate, channels) {
    const numSamples = samples.length;
    const bytesPerSample = 2; // 16-bit
    const blockAlign = channels * bytesPerSample;
    const byteRate = sampleRate * blockAlign;
    const dataSize = numSamples * bytesPerSample;
    const fileSize = 44 + dataSize;
    
    const buffer = new ArrayBuffer(fileSize);
    const view = new DataView(buffer);
    
    // wav header stuff
    let offset = 0;
    
    // RIFF chunk
    view.setUint32(offset, 0x52494646, false); offset += 4; // RIFF
    view.setUint32(offset, fileSize - 8, true); offset += 4;
    view.setUint32(offset, 0x57415645, false); offset += 4; // WAVE
    
    // fmt chunk
    view.setUint32(offset, 0x666d7420, false); offset += 4; // fmt 
    view.setUint32(offset, 16, true); offset += 4;
    view.setUint16(offset, 1, true); offset += 2; // PCM
    view.setUint16(offset, channels, true); offset += 2;
    view.setUint32(offset, sampleRate, true); offset += 4;
    view.setUint32(offset, byteRate, true); offset += 4;
    view.setUint16(offset, blockAlign, true); offset += 2;
    view.setUint16(offset, 16, true); offset += 2; // bits
    
    // data chunk
    view.setUint32(offset, 0x64617461, false); offset += 4; // data
    view.setUint32(offset, dataSize, true); offset += 4;
    
    // samples as int16
    for (let i = 0; i < numSamples; i++) {
        const sample = Math.max(-1, Math.min(1, samples[i]));
        const int16 = Math.floor(sample * 32767);
        view.setInt16(offset, int16, true);
        offset += 2;
    }
    
    return new Uint8Array(buffer);
}

/**
 * Generate test signals (for demo purposes)
 */
export async function generateTestSignal(type) {
    stopAudio();
    
    const sampleRate = 44100;
    const duration = 2; // 2 seconds
    let samples;
    let channels = 1;
    
    log(`\nGenerating ${type} test signal...`);
    
    if (type === 'sine') {
        samples = new Float32Array(sampleRate * duration);
        const freq = 440;
        for (let i = 0; i < samples.length; i++) {
            samples[i] = Math.sin(2 * Math.PI * freq * i / sampleRate) * 0.8;
        }
        log('Generated 440Hz sine wave (mono, 2s)');
    } else if (type === 'stereo') {
        channels = 2;
        samples = new Float32Array(sampleRate * duration * channels);
        const freqL = 440; // A4 in left
        const freqR = 554.37; // C#5 in right
        for (let i = 0; i < sampleRate * duration; i++) {
            samples[i * 2] = Math.sin(2 * Math.PI * freqL * i / sampleRate) * 0.8; // L
            samples[i * 2 + 1] = Math.sin(2 * Math.PI * freqR * i / sampleRate) * 0.8; // R
        }
        log('Generated stereo test (440Hz L, 554Hz R, 2s)');
    } else {
        // White noise
        samples = new Float32Array(sampleRate * duration);
        for (let i = 0; i < samples.length; i++) {
            samples[i] = (Math.random() * 2 - 1) * 0.5;
        }
        log('Generated white noise (mono, 2s)');
    }
    
    // Generate WAV file bytes
    const wavBytes = generateWavBytes(samples, sampleRate, channels);
    
    // Store as audio file and encode
    state.audioFileBytes = wavBytes;
    state.sourceFileName = `test-${type}.wav`;
    
    const { encodeAndUpdateUI } = await import('./encoder.js');
    await encodeAndUpdateUI();
}

/**
 * Start/stop recording (for demo purposes)
 */
let mediaRecorder = null;
let audioChunks = [];
let isRecording = false;

export async function startRecording() {
    if (isRecording) {
        stopRecording();
        return;
    }
    
    try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        mediaRecorder = new MediaRecorder(stream);
        audioChunks = [];
        
        mediaRecorder.ondataavailable = (event) => {
            audioChunks.push(event.data);
        };
        
        mediaRecorder.onstop = async () => {
            const audioBlob = new Blob(audioChunks, { type: 'audio/webm' });
            const arrayBuffer = await audioBlob.arrayBuffer();
            const bytes = new Uint8Array(arrayBuffer);
            
            log('Recording complete, encoding...');
            
            state.audioFileBytes = bytes;
            state.sourceFileName = 'recording.webm';
            
            const { encodeAndUpdateUI } = await import('./encoder.js');
            await encodeAndUpdateUI();
            
            // Stop all tracks
            stream.getTracks().forEach(track => track.stop());
        };
        
        mediaRecorder.start();
        isRecording = true;
        
        // Update UI
        const recordBtn = document.getElementById('recordBtn');
        if (recordBtn) {
            recordBtn.textContent = '⏹ Stop';
            recordBtn.classList.add('recording');
        }
        
        log('Recording started (click Stop to finish)...', 'info');
        
    } catch (err) {
        log(`Failed to start recording: ${err.message}`, 'error');
        console.error('Recording error:', err);
    }
}

export function stopRecording() {
    if (mediaRecorder && isRecording) {
        mediaRecorder.stop();
        isRecording = false;
        
        // Update UI
        const recordBtn = document.getElementById('recordBtn');
        if (recordBtn) {
            recordBtn.textContent = '🎤 Record';
            recordBtn.classList.remove('recording');
        }
    }
}

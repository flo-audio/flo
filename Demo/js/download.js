import { state } from './state.js';
import { log } from './ui.js';

/**
 * Download current flo™ file
 */
export function downloadFlo() {
    if (!state.floData) {
        log('No flo™ file to download', 'error');
        return;
    }
    
    const blob = new Blob([state.floData], { type: 'application/octet-stream' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'audio.flo';
    a.click();
    URL.revokeObjectURL(url);
    
    log('Downloaded audio.flo', 'success');
}

// convert back to wav and download
export function downloadWav() {
    if (!state.decodedSamples || !state.decodedSampleRate) {
        log('No audio to download', 'error');
        return;
    }
    
    const samples = state.decodedSamples;
    const sampleRate = state.decodedSampleRate;
    const channels = state.decodedChannels || 1;
    const wavData = createWav(samples, sampleRate, channels);
    
    const blob = new Blob([wavData], { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'audio.wav';
    a.click();
    URL.revokeObjectURL(url);
    
    log('Downloaded audio.wav', 'success');
}

// build a wav file from scratch because why not
function createWav(samples, sampleRate, channels) {
    const length = samples.length;
    const bytesPerSample = 2;
    const blockAlign = channels * bytesPerSample;
    const byteRate = sampleRate * blockAlign;
    const dataSize = length * bytesPerSample;
    
    const buffer = new ArrayBuffer(44 + dataSize);
    const view = new DataView(buffer);
    
    // RIFF header
    writeString(view, 0, 'RIFF');
    view.setUint32(4, 36 + dataSize, true);
    writeString(view, 8, 'WAVE');
    
    // fmt chunk
    writeString(view, 12, 'fmt ');
    view.setUint32(16, 16, true); // chunk size
    view.setUint16(20, 1, true); // PCM format
    view.setUint16(22, channels, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, byteRate, true);
    view.setUint16(32, blockAlign, true);
    view.setUint16(34, 16, true); // bits per sample
    
    // data chunk
    writeString(view, 36, 'data');
    view.setUint32(40, dataSize, true);
    
    // convert float samples to 16bit int
    let offset = 44;
    for (let i = 0; i < length; i++) {
        const sample = Math.max(-1, Math.min(1, samples[i]));
        const int16 = sample < 0 ? sample * 0x8000 : sample * 0x7FFF;
        view.setInt16(offset, int16, true);
        offset += 2;
    }
    
    return buffer;
}

function writeString(view, offset, string) {
    for (let i = 0; i < string.length; i++) {
        view.setUint8(offset + i, string.charCodeAt(i));
    }
}

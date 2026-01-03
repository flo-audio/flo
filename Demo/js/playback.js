import { state } from './state.js';
import { log } from './ui.js';

let currentSource = null;
let isPlaying = false;

/**
 * Stop any currently playing audio
 */
export function stopAudio() {
    if (currentSource) {
        try {
            currentSource.stop();
        } catch (e) {
            // Already stopped
        }
        currentSource = null;
    }
    isPlaying = false;
    updatePlayButton();
}

export function isAudioPlaying() {
    return isPlaying;
}

// play or pause, you know the drill
export function togglePlayback() {
    if (isPlaying) {
        stopAudio();
    } else {
        playAudio();
    }
}

function updatePlayButton() {
    const playBtn = document.getElementById('playBtn');
    if (playBtn) {
        if (isPlaying) {
            playBtn.innerHTML = `
                <svg class="icon" viewBox="0 0 24 24"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                Pause
            `;
        } else {
            playBtn.innerHTML = `
                <svg class="icon" viewBox="0 0 24 24"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                Play
            `;
        }
    }
}

// make noise happen
export function playAudio() {

    if (!state.decodedSamples || !state.decodedSampleRate) {
        log('No audio to play', 'error');
        return;
    }

    stopAudio();

    if (!state.audioCtx) {
        state.audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    }

    // browsers are weird about autoplay
    if (state.audioCtx.state === 'suspended') {
        state.audioCtx.resume();
    }

    const samples = state.decodedSamples;
    const sampleRate = state.decodedSampleRate;
    const channels = state.decodedChannels || 1;
    const length = Math.floor(samples.length / channels);

    const buffer = state.audioCtx.createBuffer(channels, length, sampleRate);

    // De-interleave
    for (let ch = 0; ch < channels; ch++) {
        const channelData = buffer.getChannelData(ch);
        for (let i = 0; i < length; i++) {
            channelData[i] = samples[i * channels + ch];
        }
    }

    currentSource = state.audioCtx.createBufferSource();
    currentSource.buffer = buffer;
    currentSource.connect(state.audioCtx.destination);

    currentSource.onended = () => {
        isPlaying = false;
        currentSource = null;
        updatePlayButton();
        log('Playback finished', 'info');
    };

    currentSource.start();
    isPlaying = true;
    updatePlayButton();

    log('Playing audio...', 'success');
}

// play the original source audio
export function playSourceAudio() {
    if (!state.sourceSamples) {
        log('No source audio to play', 'error');
        return;
    }

    stopAudio();

    if (!state.audioCtx) {
        state.audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    }

    if (state.audioCtx.state === 'suspended') {
        state.audioCtx.resume();
    }

    const samples = state.sourceSamples;
    const sampleRate = state.sourceSampleRate;
    const channels = state.sourceChannels;
    const length = Math.floor(samples.length / channels);

    const buffer = state.audioCtx.createBuffer(channels, length, sampleRate);

    // De-interleave
    for (let ch = 0; ch < channels; ch++) {
        const channelData = buffer.getChannelData(ch);
        for (let i = 0; i < length; i++) {
            channelData[i] = samples[i * channels + ch];
        }
    }

    currentSource = state.audioCtx.createBufferSource();
    currentSource.buffer = buffer;
    currentSource.connect(state.audioCtx.destination);

    currentSource.onended = () => {
        isPlaying = false;
        currentSource = null;
        updatePlayButton();
    };

    currentSource.start();
    isPlaying = true;
    updatePlayButton();

    log('Playing source audio...', 'success');
}

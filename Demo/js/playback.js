import { state } from './state.js';
import { log } from './ui.js';
import { 
    setVisualizerPeaks, 
    setPlaybackTime, 
    startVisualization, 
    stopVisualization, 
    drawSeekbar, 
    formatTime 
} from './visualizer.js';

let currentSource = null;
let isPlaying = false;
let startTime = 0;
let pauseTime = 0;
let duration = 0;
let seekbarInterval = null;
let waveformPeaks = null;

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
    pauseTime = 0;
    updatePlayButton();
    stopVisualization();
    stopSeekbarUpdate();
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

function startSeekbarUpdate() {
    stopSeekbarUpdate();
    
    const seekbar = document.getElementById('seekbar');
    const timeDisplay = document.getElementById('timeDisplay');
    
    if (!seekbar) return;
    
    seekbarInterval = setInterval(() => {
        if (!isPlaying || !state.audioCtx) return;
        
        const currentTime = state.audioCtx.currentTime - startTime + pauseTime;
        
        // Update visualizer with current playback time
        setPlaybackTime(currentTime, duration);
        
        drawSeekbar(seekbar, currentTime, duration, waveformPeaks);
        
        if (timeDisplay) {
            timeDisplay.textContent = `${formatTime(currentTime)} / ${formatTime(duration)}`;
        }
    }, 50);
}

function stopSeekbarUpdate() {
    if (seekbarInterval) {
        clearInterval(seekbarInterval);
        seekbarInterval = null;
    }
}

/**
 * Seek to a specific position
 */
export function seekTo(time) {
    if (!state.decodedSamples || !state.decodedSampleRate) return;
    
    const wasPlaying = isPlaying;
    stopAudio();
    pauseTime = Math.max(0, Math.min(time, duration));
    
    if (wasPlaying) {
        playAudio();
    } else {
        // Just update the seekbar
        const seekbar = document.getElementById('seekbar');
        const timeDisplay = document.getElementById('timeDisplay');
        
        if (seekbar) {
            drawSeekbar(seekbar, pauseTime, duration, waveformPeaks);
        }
        if (timeDisplay) {
            timeDisplay.textContent = `${formatTime(pauseTime)} / ${formatTime(duration)}`;
        }
    }
}

/**
 * Handle seekbar click
 */
export function handleSeekbarClick(event) {
    const seekbar = document.getElementById('seekbar');
    if (!seekbar || duration <= 0) return;
    
    const rect = seekbar.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const ratio = x / rect.width;
    const time = ratio * duration;
    
    seekTo(time);
}

/**
 * Set waveform data for seekbar and visualizer
 * @param {Array|Object} peaks - Array of peaks or waveformData object from WASM
 */
export function setWaveformPeaks(peaks) {
    // Handle both raw array and waveformData object from WASM
    if (peaks?.peaks) {
        waveformPeaks = Array.from(peaks.peaks);
        setVisualizerPeaks(peaks);
    } else if (Array.isArray(peaks)) {
        waveformPeaks = peaks;
        setVisualizerPeaks({ peaks, peaks_per_second: 50 });
    } else {
        waveformPeaks = null;
        setVisualizerPeaks(null);
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
    duration = length / sampleRate;

    const buffer = state.audioCtx.createBuffer(channels, length, sampleRate);

    // De-interleave
    for (let ch = 0; ch < channels; ch++) {
        const channelData = buffer.getChannelData(ch);
        for (let i = 0; i < length; i++) {
            channelData[i] = samples[i * channels + ch];
        }
    }

    // Set up audio graph (no analyser needed - using pre-computed peaks)
    currentSource = state.audioCtx.createBufferSource();
    currentSource.buffer = buffer;
    currentSource.connect(state.audioCtx.destination);

    currentSource.onended = () => {
        isPlaying = false;
        currentSource = null;
        pauseTime = 0;
        updatePlayButton();
        stopVisualization();
        stopSeekbarUpdate();
        
        // Final seekbar update
        const seekbar = document.getElementById('seekbar');
        const timeDisplay = document.getElementById('timeDisplay');
        if (seekbar) drawSeekbar(seekbar, duration, duration, waveformPeaks);
        if (timeDisplay) timeDisplay.textContent = `${formatTime(duration)} / ${formatTime(duration)}`;
        
        log('Playback finished', 'info');
    };

    // Start from pauseTime offset
    startTime = state.audioCtx.currentTime;
    currentSource.start(0, pauseTime);
    isPlaying = true;
    updatePlayButton();
    
    // Start visualizations
    startVisualization();
    startSeekbarUpdate();

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

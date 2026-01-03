import init from './pkg-reflo/reflo.js';
import initLibflo from './pkg-libflo/libflo.js';
import { state, hasSource } from './js/state.js';
import { encodeAndUpdateUI, scheduleReencode, scheduleMetadataUpdate } from './js/encoder.js';
import { generateTestSignal, handleFile as handleFileInternal, startRecording, stopRecording } from './js/audio.js';
import { log, drawEmptyWaveform, drawWaveform, toggleMetadataEditor, updateStats } from './js/ui.js';
import { togglePlayback, stopAudio } from './js/playback.js';
import { downloadFlo as downloadFloInternal, downloadWav as downloadWavInternal } from './js/download.js';
import { streamFromFile, playStreamedAudio, stopStreaming, debugCompareDecodeMethods } from './js/streaming.js';

async function initialize() {
    try {
        await Promise.all([init(), initLibflo()]);
        log(`flo audio converter ready`, 'success');
        drawEmptyWaveform();
        setupEventListeners();
    } catch (err) {
        log(`Failed to load WASM: ${err.message}`, 'error');
    }
}

function setupEventListeners() {
    // metadata inputs use instant update
    const metadataInputs = [
        'metaTitle', 'metaArtist', 'metaAlbum', 'metaYear', 
        'metaGenre', 'metaBpm', 'metaKey', 'metaTrack', 'metaComment'
    ];
    
    metadataInputs.forEach(id => {
        const el = document.getElementById(id);
        if (el) {
            el.addEventListener('input', () => {
                if (state.floData) {
                    scheduleMetadataUpdate(500);
                } else if (hasSource()) {
                    scheduleReencode(500);
                }
            });
        }
    });
    
    // quality slider needs full re-encode
    const qualityRange = document.getElementById('qualityRange');
    if (qualityRange) {
        qualityRange.addEventListener('input', () => {
            state.lossyQuality = parseInt(qualityRange.value);
            updateQualityLabel();
            if (hasSource()) scheduleReencode(200);
        });
    }
    
    // drag and drop
    const dropZone = document.body;
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
    });
    
    dropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        e.stopPropagation();
        const file = e.dataTransfer?.files?.[0];
        if (file) handleFile(file);
    });
    
    // redraw waveform on resize
    let resizeTimer;
    window.addEventListener('resize', () => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
            if (state.sourceSamples && state.currentDecodedAudio) {
                drawWaveform(state.sourceSamples, state.currentDecodedAudio.samples);
            } else {
                drawEmptyWaveform();
            }
        }, 100);
    });
}

// functions exposed to html onclick handlers

window.runTest = async function(type) {
    try {
        stopAudio();
        await generateTestSignal(type);
    } catch (err) {
        log(`Error: ${err.message}`, 'error');
        console.error(err);
    }
};

window.toggleRecording = async function() {
    try {
        await startRecording();
    } catch (err) {
        log(`Recording error: ${err.message}`, 'error');
    }
};

window.handleFile = function(file) {
    handleFileInternal(file);
};

window.playAudio = function() {
    togglePlayback();
};

window.stopAudio = function() {
    stopAudio();
};

window.downloadFlo = function() {
    downloadFloInternal();
};

window.downloadWav = function() {
    downloadWavInternal();
};

window.toggleMetadataEditor = function() {
    toggleMetadataEditor();
};

window.setEncodingMode = function(mode) {
    state.encodingMode = mode;
    
    document.getElementById('modeLossless').classList.toggle('active', mode === 'lossless');
    document.getElementById('modeLossy').classList.toggle('active', mode === 'lossy');
    
    const slider = document.getElementById('qualitySlider');
    if (mode === 'lossy') {
        slider.classList.remove('hidden');
    } else {
        slider.classList.add('hidden');
    }
    
    log(`Encoding mode: ${mode}`, 'info');
    if (hasSource()) scheduleReencode(100);
};

window.updateQualityLabel = function() {
    const range = document.getElementById('qualityRange');
    state.lossyQuality = parseInt(range.value);
    
    const labels = [
        'Low (~64 kbps)',
        'Medium (~128 kbps)',
        'High (~192 kbps)',
        'Very High (~256 kbps)',
        'Transparent (~320 kbps)'
    ];
    
    document.getElementById('qualityValue').textContent = labels[state.lossyQuality];
};

function updateQualityLabel() {
    window.updateQualityLabel();
}

// streaming stuff

window.streamFile = function() {
    document.getElementById('streamFileInput').click();
};

window.handleStreamFile = function(file) {
    if (file && file.name.endsWith('.flo')) {
        streamFromFile(file);
    } else {
        log('Please select a .flo file for streaming', 'error');
    }
};

window.playStreamed = function() {
    playStreamedAudio();
};

window.stopStream = function() {
    stopStreaming();
    log('Streaming stopped');
};

window.debugStreamingDecode = debugCompareDecodeMethods;

initialize();

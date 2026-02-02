import { update_flo_metadata, get_flo_info } from '../pkg-reflo/reflo.js';
import { encodeAudio } from './codec.js';
import { state } from './state.js';
import {
  log, updateStats, drawWaveform, showCards, displayMetadata, displayEncodingInfo,
  displayMetadataFromEditor, getMetadataFromEditor, populateMetadataEditor
} from './ui.js';
import { analyzeAudio, updateAnalysisPanel, hideAnalysisPanel } from './analysis.js';
import { setWaveformPeaks } from './playback.js';
import { drawSeekbar, formatTime } from './visualizer.js';

/**
 * Extract waveform peaks for seekbar visualization (fallback)
 */
function extractPeaks(samples, numPeaks = 200) {
    const samplesPerPeak = Math.floor(samples.length / numPeaks);
    const peaks = [];
    
    for (let i = 0; i < numPeaks; i++) {
        let max = 0;
        const start = i * samplesPerPeak;
        const end = Math.min(start + samplesPerPeak, samples.length);
        
        for (let j = start; j < end; j++) {
            const val = Math.abs(samples[j]);
            if (val > max) max = val;
        }
        peaks.push(max);
    }
    
    return peaks;
}

// timers for debouncing (dont spam encodes)
let reencodeTimer = null;
let metadataUpdateTimer = null;

export function scheduleReencode(delayMs = 300) {
    clearTimeout(reencodeTimer);
    clearTimeout(metadataUpdateTimer);
    reencodeTimer = setTimeout(() => {
        if (state.audioFileBytes) {
            encodeAndUpdateUI();
        }
    }, delayMs);
}

export function scheduleMetadataUpdate(delayMs = 300) {
    clearTimeout(metadataUpdateTimer);
    metadataUpdateTimer = setTimeout(() => {
        if (state.floData) {
            updateMetadataOnly();
        }
    }, delayMs);
}

export async function updateMetadataOnly() {
    if (!state.floData) {
        log('No encoded file to update metadata', 'warning');
        return;
    }

    try {
        const startTime = performance.now();
        const metadata = getMetadataFromEditor();

        // instant metadata swap, no audio touched
        const updatedFloData = await update_flo_metadata(state.floData, metadata);

        const updateTime = performance.now() - startTime;
        state.floData = updatedFloData;

        const fileInfo = await get_flo_info(updatedFloData);
        state.fileInfo = fileInfo;

        updateStats({
            sampleRate: fileInfo.sample_rate,
            channels: fileInfo.channels,
            duration: fileInfo.duration_secs,
            originalSize: Math.round(fileInfo.duration_secs * fileInfo.sample_rate * fileInfo.channels * 2),
            floSize: updatedFloData.length,
            compressionRatio: fileInfo.compression_ratio,
            encodeTime: updateTime,
            lossy: fileInfo.is_lossy,
            quality: fileInfo.lossy_quality
        });

        displayMetadata(updatedFloData);

        log(`Metadata updated in ${updateTime.toFixed(1)}ms`, 'success');

    } catch (err) {
        log(`Metadata update failed: ${err.message}`, 'error');
        console.error('Metadata update error:', err);
    }
}

export async function encodeAudioFile() {
    if (!state.audioFileBytes) {
        throw new Error('No audio file to encode');
    }

    const audioBytes = state.audioFileBytes;
    const lossy = state.encodingMode === 'lossy';
    const quality = lossy ? state.lossyQuality / 4.0 : 0.6;
    const filename = state.audioFileName || 'audio';

    // Use worker for encoding (keeps UI responsive)
    const result = await encodeAudio(audioBytes.buffer, filename, lossy, quality);

    return {
        floData: result.floData,
        decoded: result.samples,
        sampleRate: result.sampleRate,
        channels: result.channels,
        fileInfo: result.fileInfo,
        metadata: result.metadata,
        waveformData: result.waveformData
    };
}

export async function encodeAndUpdateUI() {
    if (!state.audioFileBytes) {
        log('No audio to encode', 'warning');
        return;
    }

    try {
        log('Encoding...', 'info');
        const startTime = performance.now();
        const { floData, decoded, sampleRate, channels, fileInfo, metadata, waveformData: wfData } = await encodeAudioFile();
        const encodeTime = performance.now() - startTime;

        state.floData = floData;
        state.decodedSamples = decoded;
        state.decodedSampleRate = sampleRate;
        state.decodedChannels = channels;
        state.fileInfo = fileInfo;

        showCards(['result', 'metadata', 'analysis', 'encodingInfo']);

        updateStats({
            sampleRate: fileInfo.sample_rate,
            channels: fileInfo.channels,
            duration: fileInfo.duration_secs,
            originalSize: Math.round(fileInfo.duration_secs * fileInfo.sample_rate * fileInfo.channels * 2),
            floSize: floData.length,
            compressionRatio: fileInfo.compression_ratio,
            encodeTime,
            lossy: fileInfo.is_lossy,
            quality: fileInfo.lossy_quality
        });

        drawWaveform(decoded, decoded);

        // Use waveform data from worker or extract as fallback
        let waveformData = wfData;
        if (!waveformData?.peaks || waveformData.peaks.length === 0) {
            const peaks = extractPeaks(decoded);
            waveformData = { peaks, peaks_per_second: 50 };
        }
        
        setWaveformPeaks(waveformData);
        
        // Initialize seekbar display
        const seekbar = document.getElementById('seekbar');
        const timeDisplay = document.getElementById('timeDisplay');
        const duration = fileInfo.duration_secs;
        if (seekbar) drawSeekbar(seekbar, 0, duration, waveformData.peaks);
        if (timeDisplay) timeDisplay.textContent = `0:00 / ${formatTime(duration)}`;

        if (metadata && Object.keys(metadata).length > 0) {
            displayMetadata(floData);
        } else {
            displayMetadataFromEditor();
        }
        
        // Display encoding info
        displayEncodingInfo(floData);

        // Run audio analysis in background
        try {
            const analysis = await analyzeAudio(decoded, sampleRate, channels);
            if (analysis) {
                updateAnalysisPanel(analysis);
            }
        } catch (analysisErr) {
            console.warn('Analysis failed:', analysisErr);
        }

        log(`Encoded in ${encodeTime.toFixed(1)}ms (${fileInfo.compression_ratio.toFixed(2)}x compression)`, 'success');

    } catch (err) {
        log(`Encoding failed: ${err.message}`, 'error');
        console.error('Encoding error:', err);
    }
}

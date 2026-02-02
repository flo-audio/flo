import { decodeFlo } from './codec.js';
import { state } from './state.js';
import { log, updateStats, showCards, displayMetadata, displayEncodingInfo, drawWaveform, populateMetadataEditor } from './ui.js';
import { analyzeAudio, updateAnalysisPanel } from './analysis.js';
import { setWaveformPeaks } from './playback.js';
import { drawSeekbar, formatTime } from './visualizer.js';

/**
 * Extract waveform peaks for seekbar visualization
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

/**
 * Decode and display a flo™ file
 */
export async function decodeFloFile(floBytes) {
    try {
        log('Decoding flo™ file...');

        // Save floData before sending to worker (in case buffer gets detached)
        const floData = new Uint8Array(floBytes);
        
        const startTime = performance.now();

        // Use worker for decoding (keeps UI responsive)
        const result = await decodeFlo(floData);
        const { samples, fileInfo, waveformData: wfData, metadata, encodingInfo } = result;
        const sample_rate = fileInfo.sample_rate;
        const channels = fileInfo.channels;

        const decodeTime = performance.now() - startTime;

        // stash everything
        state.floData = floData;
        state.decodedSamples = samples;
        state.decodedSampleRate = sample_rate;
        state.decodedChannels = channels;
        state.fileInfo = fileInfo;

        showCards(['result', 'metadata', 'analysis', 'encodingInfo']);

        const originalSize = Math.round(fileInfo.duration_secs * fileInfo.sample_rate * fileInfo.channels * 2);
        updateStats({
            sampleRate: fileInfo.sample_rate,
            channels: fileInfo.channels,
            duration: fileInfo.duration_secs,
            originalSize,
            floSize: floBytes.length,
            compressionRatio: fileInfo.compression_ratio,
            decodeTime,
            lossy: fileInfo.is_lossy,
            quality: fileInfo.lossy_quality
        });

        // Draw waveform: show decoded samples
        drawWaveform(samples, samples);

        // Use waveform data from worker or extract as fallback
        let waveformData = wfData;
        if (!waveformData?.peaks || waveformData.peaks.length === 0) {
            const peaks = extractPeaks(samples);
            waveformData = { peaks, peaks_per_second: 50 };
        }
        
        setWaveformPeaks(waveformData);
        
        // Initialize seekbar display
        const seekbar = document.getElementById('seekbar');
        const timeDisplay = document.getElementById('timeDisplay');
        const duration = fileInfo.duration_secs;
        if (seekbar) drawSeekbar(seekbar, 0, duration, waveformData.peaks);
        if (timeDisplay) timeDisplay.textContent = `0:00 / ${formatTime(duration)}`;

        displayMetadata(state.floData);
        displayEncodingInfo(state.floData);
        populateMetadataEditor(state.floData);

        // Run audio analysis
        try {
            const analysis = await analyzeAudio(samples, sample_rate, channels);
            if (analysis) {
                updateAnalysisPanel(analysis);
            }
        } catch (analysisErr) {
            console.warn('Analysis failed:', analysisErr);
        }

        log(`Decoded in ${decodeTime.toFixed(1)}ms (${fileInfo.compression_ratio.toFixed(2)}x compression)`, 'success');

    } catch (err) {
        log(`Decoding failed: ${err.message}`, 'error');
        console.error('Decoding error:', err);
    }
}
import { state } from './state.js';
import { log } from './ui.js';

let analysisAvailable = false;
let computeLoudness, extractWaveformPeaks, extractSpectralFingerprint;

export async function initAnalysis() {
    try {
        const libflo = await import('../pkg-libflo/libflo_audio.js');
        computeLoudness = libflo.compute_loudness_metrics;
        extractWaveformPeaks = libflo.extract_waveform_peaks_wasm;
        extractSpectralFingerprint = libflo.extract_spectral_fingerprint_wasm;
        
        if (computeLoudness && extractWaveformPeaks) {
            analysisAvailable = true;
        }
    } catch (err) {
        console.warn('Analysis functions not available:', err);
        analysisAvailable = false;
    }
    return analysisAvailable;
}

/**
 * Analyze audio and return metrics
 */
export async function analyzeAudio(samples, sampleRate, channels) {
    if (!analysisAvailable || !samples || samples.length === 0) {
        return null;
    }

    const results = {
        loudness: null,
        waveform: null,
        spectral: null,
        duration: samples.length / channels / sampleRate
    };

    try {
        // Compute loudness metrics
        if (computeLoudness) {
            results.loudness = computeLoudness(samples, channels, sampleRate);
        }
    } catch (err) {
        console.warn('Loudness analysis failed:', err);
    }

    try {
        // Extract waveform peaks
        if (extractWaveformPeaks) {
            results.waveform = extractWaveformPeaks(samples, channels, sampleRate, 100);
        }
    } catch (err) {
        console.warn('Waveform extraction failed:', err);
    }

    try {
        // Compute spectral fingerprint
        if (extractSpectralFingerprint) {
            results.spectral = extractSpectralFingerprint(samples, channels, sampleRate, null, null);
        }
    } catch (err) {
        console.warn('Spectral analysis failed:', err);
    }

    return results;
}

/**
 * Format LUFS value for display
 */
export function formatLUFS(lufs) {
    if (lufs === null || lufs === undefined || !isFinite(lufs)) return '-∞';
    if (lufs <= -70) return '-∞';
    return lufs.toFixed(1) + ' LUFS';
}

/**
 * Format dB value for display
 */
export function formatDB(db) {
    if (db === null || db === undefined || !isFinite(db)) return '-∞ dB';
    if (db <= -70) return '-∞ dB';
    return db.toFixed(1) + ' dB';
}

/**
 * Format loudness range for display
 */
export function formatLU(lu) {
    if (lu === null || lu === undefined || !isFinite(lu)) return '0 LU';
    return lu.toFixed(1) + ' LU';
}

/**
 * Get loudness level category
 */
export function getLoudnessCategory(lufs) {
    if (lufs > -9) return { label: 'Very Loud', color: '#ef4444' };
    if (lufs > -14) return { label: 'Loud', color: '#f97316' };
    if (lufs > -18) return { label: 'Normal', color: '#22c55e' };
    if (lufs > -24) return { label: 'Quiet', color: '#3b82f6' };
    return { label: 'Very Quiet', color: '#6366f1' };
}

/**
 * Draw loudness meter visualization
 */
export function drawLoudnessMeter(canvas, lufs, truePeak) {
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    
    // Background
    ctx.fillStyle = '#0d0d0d';
    ctx.fillRect(0, 0, width, height);
    
    // Meter track
    const trackY = height / 2 - 8;
    const trackH = 16;
    const padding = 20;
    const trackW = width - padding * 2;
    
    // Draw gradient track background
    const gradient = ctx.createLinearGradient(padding, 0, padding + trackW, 0);
    gradient.addColorStop(0, '#1c1c1c');
    gradient.addColorStop(1, '#1c1c1c');
    ctx.fillStyle = gradient;
    ctx.fillRect(padding, trackY, trackW, trackH);
    
    // Draw meter segments (colored zones)
    const zones = [
        { start: -60, end: -24, color: '#6366f1' },  // Very quiet
        { start: -24, end: -18, color: '#3b82f6' },  // Quiet
        { start: -18, end: -14, color: '#22c55e' },  // Normal
        { start: -14, end: -9, color: '#f97316' },   // Loud
        { start: -9, end: 0, color: '#ef4444' },     // Very loud
    ];
    
    // Draw zone backgrounds (dimmed)
    zones.forEach(zone => {
        const x1 = padding + ((zone.start + 60) / 60) * trackW;
        const x2 = padding + ((zone.end + 60) / 60) * trackW;
        ctx.fillStyle = zone.color + '20';
        ctx.fillRect(x1, trackY, x2 - x1, trackH);
    });
    
    // Draw filled meter based on LUFS
    if (lufs !== null && lufs > -60) {
        const fillWidth = ((lufs + 60) / 60) * trackW;
        const clampedWidth = Math.max(0, Math.min(fillWidth, trackW));
        
        // Determine color based on LUFS level
        let fillColor = '#6366f1';
        for (const zone of zones) {
            if (lufs >= zone.start && lufs < zone.end) {
                fillColor = zone.color;
                break;
            }
        }
        if (lufs >= 0) fillColor = '#ef4444';
        
        ctx.fillStyle = fillColor;
        ctx.fillRect(padding, trackY, clampedWidth, trackH);
    }
    
    // Draw true peak marker
    if (truePeak !== null && truePeak > -60) {
        const peakX = padding + ((truePeak + 60) / 60) * trackW;
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(peakX, trackY - 4);
        ctx.lineTo(peakX, trackY + trackH + 4);
        ctx.stroke();
    }
    
    // Draw scale labels
    ctx.fillStyle = '#737373';
    ctx.font = '10px -apple-system, sans-serif';
    ctx.textAlign = 'center';
    
    const labels = [-60, -40, -24, -14, 0];
    labels.forEach(db => {
        const x = padding + ((db + 60) / 60) * trackW;
        ctx.fillText(`${db}`, x, trackY - 8);
    });
}

/**
 * Draw frequency spectrum bars
 */
export function drawSpectrumBars(canvas, energyProfile) {
    if (!canvas || !energyProfile) {
        console.log('Spectrum bars: missing canvas or energyProfile', { canvas: !!canvas, energyProfile: !!energyProfile });
        return;
    }
    
    // Convert to regular array if it's a typed array
    const profile = Array.isArray(energyProfile) ? energyProfile : Array.from(energyProfile);
    
    console.log('Drawing spectrum bars:', { length: profile.length, values: profile.slice(0, 4) });
    
    if (profile.length === 0) {
        console.log('Spectrum bars: empty profile');
        return;
    }
    
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    if (width === 0 || height === 0) {
        console.log('Spectrum bars: canvas has no size', { width, height });
        return;
    }
    
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    
    // Background
    ctx.fillStyle = '#0d0d0d';
    ctx.fillRect(0, 0, width, height);
    
    const numBars = profile.length;
    const barGap = 2;
    const barWidth = (width - (numBars + 1) * barGap) / numBars;
    const maxHeight = height - 30;
    
    // Draw bars
    profile.forEach((energy, i) => {
        const x = barGap + i * (barWidth + barGap);
        const barHeight = (energy / 255) * maxHeight;
        const y = height - 20 - barHeight;
        
        // Gradient based on frequency
        const hue = 220 - (i / numBars) * 60; // Blue to purple
        ctx.fillStyle = `hsl(${hue}, 70%, 55%)`;
        ctx.fillRect(x, y, barWidth, barHeight);
    });
    
    // Frequency labels
    ctx.fillStyle = '#737373';
    ctx.font = '9px -apple-system, sans-serif';
    ctx.textAlign = 'center';
    
    const freqLabels = ['63', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];
    const labelIndices = [0, 2, 4, 6, 8, 10, 12, 14, 15];
    
    labelIndices.forEach((idx, i) => {
        if (idx < numBars && i < freqLabels.length) {
            const x = barGap + idx * (barWidth + barGap) + barWidth / 2;
            ctx.fillText(freqLabels[i], x, height - 4);
        }
    });
}

/**
 * Update the analysis panel UI
 */
export function updateAnalysisPanel(analysis) {
    const panel = document.getElementById('analysisCard');
    if (!panel) return;
    
    panel.classList.remove('hidden');
    
    // Update loudness values
    if (analysis?.loudness) {
        const loudness = analysis.loudness;
        const integratedEl = document.getElementById('analysisIntegrated');
        const rangeEl = document.getElementById('analysisRange');
        const peakEl = document.getElementById('analysisPeak');
        
        if (integratedEl) integratedEl.textContent = formatLUFS(loudness.integrated_lufs);
        if (rangeEl) rangeEl.textContent = formatLU(loudness.loudness_range_lu);
        if (peakEl) peakEl.textContent = formatDB(loudness.true_peak_dbtp);
        
        // Update meter
        const meterCanvas = document.getElementById('loudnessMeter');
        if (meterCanvas) {
            drawLoudnessMeter(meterCanvas, loudness.integrated_lufs, loudness.true_peak_dbtp);
        }
        
        // Update category badge
        const category = getLoudnessCategory(loudness.integrated_lufs);
        const categoryEl = document.getElementById('analysisCategory');
        if (categoryEl) {
            categoryEl.textContent = category.label;
            categoryEl.style.color = category.color;
        }
    }
    
    // Update spectrum visualization
    if (analysis?.spectral?.energy_profile) {
        const spectrumCanvas = document.getElementById('spectrumBars');
        if (spectrumCanvas) {
            drawSpectrumBars(spectrumCanvas, analysis.spectral.energy_profile);
        }
    }
    
    // Update duration
    if (analysis?.duration) {
        const durationEl = document.getElementById('analysisDuration');
        if (durationEl) {
            const mins = Math.floor(analysis.duration / 60);
            const secs = (analysis.duration % 60).toFixed(1);
            durationEl.textContent = mins > 0 ? `${mins}:${secs.padStart(4, '0')}` : `${secs}s`;
        }
    }
}

/**
 * Hide the analysis panel
 */
export function hideAnalysisPanel() {
    const panel = document.getElementById('analysisCard');
    if (panel) {
        panel.classList.add('hidden');
    }
}

export { analysisAvailable };

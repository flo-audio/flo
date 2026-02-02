// Visualizer using pre-computed waveform peaks from flo metadata (WASM)
// Animates peaks based on current playback position - no Web Audio FFT needed

import { format_time, format_time_ms } from '../pkg-libflo/libflo_audio.js';

let animationId = null;
let waveformPeaks = null;
let peaksPerSecond = 50;
let currentPlaybackTime = 0;
let duration = 0;
let isPlaying = false;

/**
 * Set the waveform peaks data for visualization
 * @param {Object} waveformData - From get_waveform_data() { peaks, peaks_per_second, channels }
 */
export function setVisualizerPeaks(waveformData) {
    if (waveformData?.peaks) {
        waveformPeaks = Array.from(waveformData.peaks);
        peaksPerSecond = waveformData.peaks_per_second || 50;
    } else {
        waveformPeaks = null;
    }
}

/**
 * Update playback time for the visualizer
 */
export function setPlaybackTime(time, dur) {
    currentPlaybackTime = time;
    duration = dur;
}

/**
 * Start the peak-based visualization loop
 * Uses pre-computed waveform peaks, animated at playback position
 */
export function startVisualization() {
    if (animationId) return;
    isPlaying = true;
    
    const canvas = document.getElementById('liveVisualizer');
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;
    
    function draw() {
        animationId = requestAnimationFrame(draw);
        if (!isPlaying) {
            cancelAnimationFrame(animationId);
            animationId = null;
            return;
        }
        
        const width = canvas.clientWidth;
        const height = canvas.clientHeight;
        
        canvas.width = width * dpr;
        canvas.height = height * dpr;
        ctx.scale(dpr, dpr);
        
        // Clear
        ctx.fillStyle = '#0d0d0d';
        ctx.fillRect(0, 0, width, height);
        
        if (!waveformPeaks || waveformPeaks.length === 0) {
            // No peaks - draw idle line
            ctx.strokeStyle = '#333';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(0, height / 2);
            ctx.lineTo(width, height / 2);
            ctx.stroke();
            return;
        }
        
        // Show peaks centered around current playback position
        const barCount = 64;
        const barGap = 2;
        const barWidth = (width - (barCount + 1) * barGap) / barCount;
        const maxHeight = height - 10;
        const centerY = height / 2;
        
        // Find peak index at current time
        const currentPeakIndex = Math.floor(currentPlaybackTime * peaksPerSecond);
        const halfBars = Math.floor(barCount / 2);
        const startIndex = currentPeakIndex - halfBars;
        
        for (let i = 0; i < barCount; i++) {
            const peakIndex = startIndex + i;
            let value = 0;
            
            if (peakIndex >= 0 && peakIndex < waveformPeaks.length) {
                value = Math.abs(waveformPeaks[peakIndex]) || 0;
            }
            
            const barHeight = value * maxHeight;
            const x = barGap + i * (barWidth + barGap);
            
            // Color: played (left of center) blue, upcoming (right) purple/dim
            const distFromCenter = Math.abs(i - halfBars);
            const isBehind = i < halfBars;
            
            if (isBehind) {
                const fade = 1 - (distFromCenter / halfBars) * 0.5;
                const lightness = 45 + value * 20;
                ctx.fillStyle = `hsla(220, 80%, ${lightness}%, ${fade})`;
            } else {
                const fade = 1 - (distFromCenter / halfBars) * 0.7;
                const hue = 260 - (distFromCenter / halfBars) * 40;
                const lightness = 35 + value * 15;
                ctx.fillStyle = `hsla(${hue}, 60%, ${lightness}%, ${fade})`;
            }
            
            ctx.fillRect(x, centerY - barHeight / 2, barWidth, barHeight);
        }
        
        // Center playhead line
        const centerX = barGap + halfBars * (barWidth + barGap) + barWidth / 2;
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(centerX - 1, 0, 2, height);
    }
    
    draw();
}

/**
 * Stop the visualization
 */
export function stopVisualization() {
    isPlaying = false;
    if (animationId) {
        cancelAnimationFrame(animationId);
        animationId = null;
    }
    
    // Draw static state showing peaks at current position
    const canvas = document.getElementById('liveVisualizer');
    if (canvas) {
        const ctx = canvas.getContext('2d');
        const dpr = window.devicePixelRatio || 1;
        const width = canvas.clientWidth;
        const height = canvas.clientHeight;
        
        canvas.width = width * dpr;
        canvas.height = height * dpr;
        ctx.scale(dpr, dpr);
        
        ctx.fillStyle = '#0d0d0d';
        ctx.fillRect(0, 0, width, height);
        
        if (waveformPeaks && waveformPeaks.length > 0) {
            const barCount = 64;
            const barGap = 2;
            const barWidth = (width - (barCount + 1) * barGap) / barCount;
            const maxHeight = height - 10;
            const centerY = height / 2;
            
            const currentPeakIndex = Math.floor(currentPlaybackTime * peaksPerSecond);
            const halfBars = Math.floor(barCount / 2);
            const startIndex = currentPeakIndex - halfBars;
            
            ctx.fillStyle = '#333';
            for (let i = 0; i < barCount; i++) {
                const peakIndex = startIndex + i;
                let value = 0;
                if (peakIndex >= 0 && peakIndex < waveformPeaks.length) {
                    value = Math.abs(waveformPeaks[peakIndex]) || 0;
                }
                const barHeight = value * maxHeight;
                const x = barGap + i * (barWidth + barGap);
                ctx.fillRect(x, centerY - barHeight / 2, barWidth, barHeight);
            }
        } else {
            ctx.strokeStyle = '#1c1c1c';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(0, height / 2);
            ctx.lineTo(width, height / 2);
            ctx.stroke();
        }
    }
}

/**
 * Draw playback progress on a seekbar canvas
 */
export function drawSeekbar(canvas, currentTime, dur, seekbarPeaks = null) {
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    
    ctx.fillStyle = '#1c1c1c';
    ctx.fillRect(0, 0, width, height);
    
    const peaks = seekbarPeaks || waveformPeaks;
    
    if (peaks && peaks.length > 0) {
        const peakArray = Array.isArray(peaks) ? peaks : Array.from(peaks);
        const centerY = height / 2;
        const maxHeight = height * 0.8;
        const playedWidth = dur > 0 ? (currentTime / dur) * width : 0;
        
        // Played portion
        ctx.fillStyle = '#3b82f6';
        for (let i = 0; i < peakArray.length; i++) {
            const x = (i / peakArray.length) * width;
            if (x > playedWidth) break;
            const peak = Math.abs(peakArray[i]) || 0;
            const barHeight = peak * maxHeight;
            ctx.fillRect(x, centerY - barHeight / 2, Math.max(1, width / peakArray.length - 0.5), barHeight);
        }
        
        // Unplayed portion
        ctx.fillStyle = '#4b5563';
        for (let i = 0; i < peakArray.length; i++) {
            const x = (i / peakArray.length) * width;
            if (x <= playedWidth) continue;
            const peak = Math.abs(peakArray[i]) || 0;
            const barHeight = peak * maxHeight;
            ctx.fillRect(x, centerY - barHeight / 2, Math.max(1, width / peakArray.length - 0.5), barHeight);
        }
    } else {
        const progress = dur > 0 ? currentTime / dur : 0;
        ctx.fillStyle = '#3b82f6';
        ctx.fillRect(0, 0, width * progress, height);
    }
    
    // Playhead
    if (dur > 0) {
        const playheadX = (currentTime / dur) * width;
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(playheadX - 1, 0, 2, height);
    }
}

/**
 * Format time using WASM function
 */
export function formatTime(seconds) {
    try {
        return format_time(seconds);
    } catch (e) {
        // Fallback if WASM not loaded yet
        if (!isFinite(seconds) || seconds < 0) return '0:00';
        const mins = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${mins}:${secs.toString().padStart(2, '0')}`;
    }
}

/**
 * Format time in milliseconds using WASM
 */
export function formatTimeMs(ms) {
    try {
        return format_time_ms(ms);
    } catch (e) {
        return formatTime(ms / 1000);
    }
}

// Legacy - no longer using Web Audio AnalyserNode
export function initVisualizer(audioCtx) { return null; }
export function getAnalyser() { return null; }
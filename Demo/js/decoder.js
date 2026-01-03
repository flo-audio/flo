import { decode_flo_to_samples, get_flo_file_info, get_flo_metadata_json } from '../pkg-reflo/reflo.js';
import { state } from './state.js';
import { log, updateStats, showCards, displayMetadata, drawWaveform, populateMetadataEditor } from './ui.js';

/**
 * Decode and display a flo™ file
 */
export async function decodeFloFile(floBytes) {
    try {
        log('Decoding flo™ file...');
        
        const startTime = performance.now();
        
        const fileInfo = get_flo_file_info(floBytes);
        
        const { samples, sampleRate, channels } = decode_flo_to_samples(floBytes);
        
        const decodeTime = performance.now() - startTime;
        
        // stash everything
        state.floData = floBytes;
        state.decodedSamples = samples;
        state.decodedSampleRate = sampleRate;
        state.decodedChannels = channels;
        state.fileInfo = fileInfo;
        
        showCards(['result', 'metadata']);
        
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
        
        displayMetadata(floBytes);
        populateMetadataEditor(floBytes);
        
        log(`Decoded in ${decodeTime.toFixed(1)}ms (${fileInfo.compression_ratio.toFixed(2)}x compression)`, 'success');
        
    } catch (err) {
        log(`Decoding failed: ${err.message}`, 'error');
        console.error('Decoding error:', err);
    }
}

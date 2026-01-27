import {
  encode_audio_to_flo,
  decode_flo_to_samples,
  get_flo_info,
  read_flo_metadata,
  update_flo_metadata,
  strip_flo_metadata
} from '../pkg-reflo/reflo.js';
import { state } from './state.js';
import {
  log, updateStats, drawWaveform, showCards, displayMetadata,
  displayMetadataFromEditor, getMetadataFromEditor, populateMetadataEditor
} from './ui.js';

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
    const level = 5;

    await new Promise(resolve => setTimeout(resolve, 0));
    const floData = await encode_audio_to_flo(audioBytes, lossy, quality, level);
    const fileInfo = await get_flo_info(floData);
    const [samples, sample_rate, channels] = await decode_flo_to_samples(floData);
    const metadata = await read_flo_metadata(floData);

    return {
        floData,
        decoded: samples,
        sampleRate: sample_rate,
        channels,
        fileInfo,
        metadata
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
        const { floData, decoded, sampleRate, channels, fileInfo, metadata } = await encodeAudioFile();
        const encodeTime = performance.now() - startTime;

        state.floData = floData;
        state.decodedSamples = decoded;
        state.decodedSampleRate = sampleRate;
        state.decodedChannels = channels;
        state.fileInfo = fileInfo;

        showCards(['result', 'metadata']);

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

        if (metadata && Object.keys(metadata).length > 0) {
            displayMetadata(floData);
        } else {
            displayMetadataFromEditor();
        }

        log(`Encoded in ${encodeTime.toFixed(1)}ms (${fileInfo.compression_ratio.toFixed(2)}x compression)`, 'success');

    } catch (err) {
        log(`Encoding failed: ${err.message}`, 'error');
        console.error('Encoding error:', err);
    }
}

import init_libflo, {
    decode,
    encode,
    encode_lossy,
    info,
    get_metadata,
    get_waveform_data,
    get_cover_art,
    validate
} from '../pkg-libflo/libflo_audio.js';

import init_reflo, {
    encode_audio_to_flo,
    decode_flo_to_samples,
    get_flo_info,
    get_encoding_info
} from '../pkg-reflo/reflo.js';

let libfloReady = false;
let refloReady = false;

// Initialize WASM modules
async function initWasm() {
    try {
        await init_libflo();
        libfloReady = true;
    } catch (e) {
        console.error('Failed to init libflo:', e);
    }
    
    try {
        await init_reflo();
        refloReady = true;
    } catch (e) {
        console.error('Failed to init reflo:', e);
    }
    
    return { libfloReady, refloReady };
}

// Handle messages from main thread
self.onmessage = async function(e) {
    const { id, action, payload } = e.data;
    
    try {
        let result;
        
        switch (action) {
            case 'init':
                result = await initWasm();
                break;
                
            case 'encode': {
                // Encode audio file to flo
                const { audioBytes, filename, lossy, quality } = payload;
                const level = 5; // compression level for lossless
                const floData = encode_audio_to_flo(
                    new Uint8Array(audioBytes),
                    lossy || false,
                    quality || 0.6,
                    level
                );
                const fileInfo = get_flo_info(floData);
                const samples = decode(floData);
                const waveformData = get_waveform_data(floData);
                const metadata = get_metadata(floData);
                
                result = {
                    floData: floData.buffer,
                    samples: samples.buffer,
                    fileInfo,
                    waveformData,
                    metadata,
                    sampleRate: fileInfo.sample_rate,
                    channels: fileInfo.channels
                };
                // Transfer buffers for performance
                self.postMessage({ id, success: true, result }, [result.floData, result.samples]);
                return;
            }
            
            case 'decode': {
                // Decode flo file
                const { floBytes } = payload;
                const floData = new Uint8Array(floBytes);
                const fileInfo = info(floData);
                const samples = decode(floData);
                const waveformData = get_waveform_data(floData);
                const metadata = get_metadata(floData);
                const encodingInfo = get_encoding_info(floData);
                
                result = {
                    samples: samples.buffer,
                    fileInfo: {
                        sample_rate: fileInfo.sample_rate,
                        channels: fileInfo.channels,
                        bit_depth: fileInfo.bit_depth,
                        duration_secs: fileInfo.duration_secs,
                        compression_ratio: fileInfo.compression_ratio,
                        is_lossy: fileInfo.is_lossy,
                        lossy_quality: fileInfo.lossy_quality,
                        crc_valid: fileInfo.crc_valid
                    },
                    waveformData,
                    metadata,
                    encodingInfo
                };
                self.postMessage({ id, success: true, result }, [result.samples]);
                return;
            }
            
            case 'getInfo': {
                const { floBytes } = payload;
                const fileInfo = info(new Uint8Array(floBytes));
                result = {
                    sample_rate: fileInfo.sample_rate,
                    channels: fileInfo.channels,
                    bit_depth: fileInfo.bit_depth,
                    duration_secs: fileInfo.duration_secs,
                    compression_ratio: fileInfo.compression_ratio,
                    is_lossy: fileInfo.is_lossy,
                    lossy_quality: fileInfo.lossy_quality,
                    crc_valid: fileInfo.crc_valid
                };
                break;
            }
            
            case 'getMetadata': {
                const { floBytes } = payload;
                result = get_metadata(new Uint8Array(floBytes));
                break;
            }
            
            case 'getWaveform': {
                const { floBytes } = payload;
                result = get_waveform_data(new Uint8Array(floBytes));
                break;
            }
            
            case 'getEncodingInfo': {
                const { floBytes } = payload;
                result = get_encoding_info(new Uint8Array(floBytes));
                break;
            }
            
            case 'validate': {
                const { floBytes } = payload;
                result = validate(new Uint8Array(floBytes));
                break;
            }
            
            default:
                throw new Error(`Unknown action: ${action}`);
        }
        
        self.postMessage({ id, success: true, result });
        
    } catch (error) {
        self.postMessage({ id, success: false, error: error.message || String(error) });
    }
};

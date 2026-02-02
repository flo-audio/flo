let worker = null;
let messageId = 0;
let pendingRequests = new Map();
let initPromise = null;

/**
 * Initialize the codec worker
 */
export async function initWorker() {
    if (initPromise) return initPromise;
    
    initPromise = new Promise((resolve, reject) => {
        try {
            worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
            
            worker.onmessage = (e) => {
                const { id, success, result, error } = e.data;
                const pending = pendingRequests.get(id);
                
                if (pending) {
                    pendingRequests.delete(id);
                    if (success) {
                        pending.resolve(result);
                    } else {
                        pending.reject(new Error(error));
                    }
                }
            };
            
            worker.onerror = (e) => {
                console.error('Worker error:', e);
            };
            
            // Initialize WASM in the worker
            sendMessage('init').then(resolve).catch(reject);
            
        } catch (e) {
            reject(e);
        }
    });
    
    return initPromise;
}

/**
 * Send a message to the worker and wait for response
 */
function sendMessage(action, payload = {}, transfer = []) {
    return new Promise((resolve, reject) => {
        const id = ++messageId;
        pendingRequests.set(id, { resolve, reject });
        worker.postMessage({ id, action, payload }, transfer);
    });
}

/**
 * Encode audio file to flo format
 * @param {ArrayBuffer} audioBytes - Raw audio file bytes
 * @param {string} filename - Original filename
 * @param {boolean} lossy - Use lossy compression
 * @param {number} quality - Quality 0.0-1.0
 * @returns {Promise<{floData, samples, fileInfo, waveformData, metadata, sampleRate, channels}>}
 */
export async function encodeAudio(audioBytes, filename, lossy = false, quality = 0.6) {
    await initWorker();
    // Copy the buffer instead of transferring to avoid detaching the original
    const result = await sendMessage('encode', { 
        audioBytes: audioBytes instanceof ArrayBuffer ? audioBytes.slice(0) : audioBytes.buffer.slice(0), 
        filename, 
        lossy, 
        quality 
    }, []);
    
    // Convert transferred buffers back to typed arrays
    return {
        ...result,
        floData: new Uint8Array(result.floData),
        samples: new Float32Array(result.samples)
    };
}

/**
 * Decode flo file to samples
 * @param {Uint8Array} floBytes - Flo file bytes
 * @returns {Promise<{samples, fileInfo, waveformData, metadata, encodingInfo}>}
 */
export async function decodeFlo(floBytes) {
    await initWorker();
    // Don't transfer input buffer - we may need it later
    const result = await sendMessage('decode', { floBytes: floBytes.buffer.slice(0) }, []);
    
    return {
        ...result,
        samples: new Float32Array(result.samples)
    };
}

/**
 * Get flo file info without decoding
 */
export async function getFloInfo(floBytes) {
    await initWorker();
    return sendMessage('getInfo', { floBytes });
}

/**
 * Get metadata from flo file
 */
export async function getFloMetadata(floBytes) {
    await initWorker();
    return sendMessage('getMetadata', { floBytes });
}

/**
 * Get waveform data from flo file
 */
export async function getFloWaveform(floBytes) {
    await initWorker();
    return sendMessage('getWaveform', { floBytes });
}

/**
 * Get encoding info from flo file
 */
export async function getFloEncodingInfo(floBytes) {
    await initWorker();
    return sendMessage('getEncodingInfo', { floBytes });
}

/**
 * Validate flo file integrity
 */
export async function validateFlo(floBytes) {
    await initWorker();
    return sendMessage('validate', { floBytes });
}

/**
 * Check if worker is ready
 */
export function isWorkerReady() {
    return initPromise !== null;
}

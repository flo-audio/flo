/**
 * Centralized application state
 */
export const state = {
    // raw bytes of whatever audio file got dropped in
    audioFileBytes: null,
    sourceFileName: null,
    
    // the encoded flo goodness
    floData: null,
    
    // Decoded audio for playback
    decodedSamples: null,
    decodedSampleRate: 44100,
    decodedChannels: 1,
    
    // info about the flo file
    fileInfo: null,
    
    // how we're encoding
    encodingMode: 'lossless',
    lossyQuality: 2, // 0=potato, 1=ok, 2=good, 3=great, 4=overkill
    
    // web audio context for playback
    audioCtx: null,
};

/**
 * Check if we have source audio
 */
export function hasSource() {
    return state.audioFileBytes !== null && state.audioFileBytes.length > 0;
}

/**
 * Reset state
 */
export function clearSource() {
    state.audioFileBytes = null;
    state.sourceFileName = null;
    state.floData = null;
    state.decodedSamples = null;
    state.fileInfo = null;
}

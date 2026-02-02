import { state } from './state.js';
import { get_metadata, get_cover_art, get_synced_lyrics, get_section_markers, get_waveform_data } from '../pkg-libflo/libflo_audio.js';
import { get_encoding_info } from '../pkg-reflo/reflo.js';
import { formatTimeMs } from './visualizer.js';

// dom elements (lazy loaded)
let elements = null;

function getElements() {
    if (!elements) {
        elements = {
            output: document.getElementById('output'),
            canvas: document.getElementById('visualizer'),
            statsCard: document.getElementById('statsCard'),
            waveformCard: document.getElementById('waveformCard'),
            playbackCard: document.getElementById('playbackCard'),
            metadataCard: document.getElementById('metadataCard'),
            metadataContent: document.getElementById('metadataContent'),
            encodingInfoCard: document.getElementById('encodingInfoCard'),
        };
        elements.ctx = elements.canvas?.getContext('2d');
    }
    return elements;
}

const COLORS = {
    bg: '#0d0d0d',
    accent: '#3b82f6',
    success: '#22c55e',
    grid: '#1c1c1c'
};

// log a message to the output panel
export function log(message, type = 'info') {
    const { output } = getElements();
    if (!output) return;
    
    const line = document.createElement('div');
    line.className = type;
    line.textContent = message;
    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
}

export function clearLog() {
    const { output } = getElements();
    if (output) output.innerHTML = '';
}

export function updateStats(stats) {
    const frameType = stats.lossy ? 'Lossy (Transform)' : 'Lossless (ALPC)';
    const compression = `${stats.compressionRatio.toFixed(2)}x`;
    const channels = stats.channels === 1 ? 'Mono' : stats.channels === 2 ? 'Stereo' : `${stats.channels}ch`;
    const size = `${(stats.floSize / 1024).toFixed(1)} KB`;
    
    document.getElementById('statFrameType').textContent = frameType;
    document.getElementById('statCompression').textContent = compression;
    document.getElementById('statChannels').textContent = channels;
    document.getElementById('statSize').textContent = size;
}

export function showCards(cardNames = ['result', 'metadata']) {
    const { statsCard, waveformCard, playbackCard, metadataCard, encodingInfoCard } = getElements();
    const analysisCard = document.getElementById('analysisCard');
    
    statsCard?.classList.remove('hidden');
    waveformCard?.classList.remove('hidden');
    playbackCard?.classList.remove('hidden');
    
    if (cardNames.includes('metadata')) {
        metadataCard?.classList.remove('hidden');
    }
    
    if (cardNames.includes('analysis')) {
        analysisCard?.classList.remove('hidden');
    }
    
    if (cardNames.includes('encodingInfo')) {
        encodingInfoCard?.classList.remove('hidden');
    }
}

/**
 * Draw waveform visualization
 */
export function drawWaveform(original, decoded) {
    const { canvas, ctx } = getElements();
    if (!canvas || !ctx) return;
    
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth * dpr;
    const h = canvas.clientHeight * dpr;
    
    canvas.width = w;
    canvas.height = h;
    ctx.scale(dpr, dpr);
    
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    // Clear
    ctx.fillStyle = COLORS.bg;
    ctx.fillRect(0, 0, width, height);
    
    // center line
    ctx.strokeStyle = COLORS.grid;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();
    
    if (!original || original.length === 0) return;
    
    // original in blue
    ctx.strokeStyle = COLORS.accent;
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let i = 0; i < width; i++) {
        const idx = Math.floor((i / width) * original.length);
        let val = original[idx] || 0;
        if (!isFinite(val)) val = 0;
        val = Math.max(-1, Math.min(1, val));
        const y = (1 - (val * 0.4 + 0.5)) * height;
        if (i === 0) ctx.moveTo(i, y);
        else ctx.lineTo(i, y);
    }
    ctx.stroke();
    
    if (!decoded || decoded.length === 0) return;
    
    // decoded in green
    ctx.strokeStyle = COLORS.success;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let i = 0; i < width; i++) {
        const idx = Math.floor((i / width) * decoded.length);
        let val = decoded[idx] || 0;
        if (!isFinite(val)) val = 0;
        val = Math.max(-1, Math.min(1, val));
        const y = (1 - (val * 0.4 + 0.5)) * height;
        if (i === 0) ctx.moveTo(i, y);
        else ctx.lineTo(i, y);
    }
    ctx.stroke();
}

// draw the placeholder waveform
export function drawEmptyWaveform() {
    const { canvas, ctx } = getElements();
    if (!canvas || !ctx) return;
    
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth * dpr;
    const h = canvas.clientHeight * dpr;
    
    canvas.width = w;
    canvas.height = h;
    ctx.scale(dpr, dpr);
    
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    
    ctx.fillStyle = COLORS.bg;
    ctx.fillRect(0, 0, width, height);
    
    ctx.strokeStyle = COLORS.grid;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();
    
    ctx.fillStyle = '#333';
    ctx.font = '14px -apple-system, BlinkMacSystemFont, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('Load audio to visualize', width / 2, height / 2 - 10);
}

// fill in the metadata editor from flo file
export function populateMetadataEditor(floData) {
    try {
        const meta = get_metadata(floData);
        if (!meta) return;
        
        if (meta.title) document.getElementById('metaTitle').value = meta.title;
        if (meta.artist) document.getElementById('metaArtist').value = meta.artist;
        if (meta.album) document.getElementById('metaAlbum').value = meta.album;
        if (meta.year) document.getElementById('metaYear').value = meta.year;
        if (meta.genre) document.getElementById('metaGenre').value = meta.genre;
        if (meta.bpm) document.getElementById('metaBpm').value = meta.bpm;
        if (meta.key) document.getElementById('metaKey').value = meta.key;
        if (meta.track_number) document.getElementById('metaTrack').value = meta.track_number;
        
        // Handle comment - could be array or string
        if (meta.comments && meta.comments.length > 0) {
            const comment = meta.comments[0].text || meta.comments[0];
            document.getElementById('metaComment').value = comment;
        }
        
        log('Metadata loaded into editor', 'success');
    } catch (err) {
        console.warn('Failed to populate metadata editor:', err);
    }
}

// show metadata from flo data
export function displayMetadata(floData) {
    const { metadataCard, metadataContent } = getElements();
    if (!metadataCard || !metadataContent) {
        console.warn('displayMetadata: Missing metadataCard or metadataContent elements');
        return;
    }
    
    try {
        const meta = get_metadata(floData);
        displayMetadataObject(meta, floData);
    } catch (err) {
        console.warn('Failed to display metadata:', err);
        // Fall back to editor metadata
        displayMetadataFromEditor();
    }
}

// show metadata from the editor form when we dont have a flo file
export function displayMetadataFromEditor() {
    const meta = getMetadataFromEditor();
    displayMetadataObject(meta, null);
}

// actually render the metadata into the card
function displayMetadataObject(meta, floData) {
    const { metadataCard, metadataContent } = getElements();
    if (!metadataCard || !metadataContent) return;
    
    if (!meta || Object.keys(meta).length === 0) {
        metadataCard.classList.add('hidden');
        return;
    }
    
    let html = '';
    
    if (meta.title) html += metaRow('Title', meta.title);
    if (meta.artist) html += metaRow('Artist', meta.artist);
    if (meta.album) html += metaRow('Album', meta.album);
    if (meta.year) html += metaRow('Year', meta.year);
    if (meta.genre) html += metaRow('Genre', meta.genre);
    if (meta.bpm) html += metaRow('BPM', meta.bpm);
    if (meta.key) html += metaRow('Key', meta.key);
    if (meta.track_number) html += metaRow('Track', meta.track_number);
    
    // cover art and other flo-only stuff
    if (floData) {
        try {
            const coverData = get_cover_art(floData);
            if (coverData && coverData.length > 0) {
                const blob = new Blob([coverData], { type: 'image/jpeg' });
                const url = URL.createObjectURL(blob);
                html += `<div class="meta-row"><span class="meta-label">Cover</span><img src="${url}" class="cover-art" /></div>`;
            }
        } catch (e) {}
        
        // synced lyrics
        try {
            const lyrics = get_synced_lyrics(floData);
            if (lyrics && lyrics.lines && lyrics.lines.length > 0) {
                const preview = lyrics.lines.slice(0, 5).map(l => 
                    `${formatTimeMs(l.start_time)}: ${l.text}`
                ).join('\n');
                html += `<div class="meta-row"><span class="meta-label">Synced Lyrics</span><pre class="lyrics-preview">${preview}${lyrics.lines.length > 5 ? '\n...' : ''}</pre></div>`;
            }
        } catch (e) {}
        
        // section markers
        try {
            const sections = get_section_markers(floData);
            if (sections && sections.length > 0) {
                const list = sections.map(s => 
                    `${formatTimeMs(s.start_time)} - ${s.label || s.section_type}`
                ).join('\n');
                html += `<div class="meta-row"><span class="meta-label">Sections</span><pre class="sections-list">${list}</pre></div>`;
            }
        } catch (e) {}
    }
    
    if (meta.comments && meta.comments.length > 0) {
        const comment = meta.comments[0].text || meta.comments[0];
        html += metaRow('Comment', comment);
    }
    
    if (html) {
        metadataContent.innerHTML = html;
        metadataCard.classList.remove('hidden');
    } else {
        metadataCard.classList.add('hidden');
    }
}

// display encoding info from flo data
export function displayEncodingInfo(floData) {
    const { encodingInfoCard } = getElements();
    if (!encodingInfoCard) return;
    
    try {
        const info = get_encoding_info(floData);
        
        // Update UI elements
        const setField = (id, value) => {
            const el = document.getElementById(id);
            if (el) el.textContent = value || '–';
        };
        
        if (info) {
            setField('infoFilename', info.originalFilename);
            setField('infoSettings', info.encoderSettings);
            setField('infoVersion', info.encoderVersion);
            setField('infoEncodingTime', formatEncodingTime(info.encodingTime));
            setField('infoSourceFormat', info.sourceFormat);
            setField('infoTaggingTime', formatEncodingTime(info.taggingTime));
            setField('infoEncodedBy', info.encodedBy);
            
            // Show card if any field has data
            const hasData = info.originalFilename || info.encoderSettings || 
                           info.encoderVersion || info.encodingTime || 
                           info.sourceFormat || info.encodedBy;
            encodingInfoCard.classList.toggle('hidden', !hasData);
        } else {
            encodingInfoCard.classList.add('hidden');
        }
    } catch (err) {
        console.warn('Failed to get encoding info:', err);
        encodingInfoCard.classList.add('hidden');
    }
}

// Format ISO datetime to more readable format
function formatEncodingTime(isoString) {
    if (!isoString) return null;
    try {
        const date = new Date(isoString);
        if (isNaN(date.getTime())) return isoString;
        return date.toLocaleString();
    } catch {
        return isoString;
    }
}

// hide encoding info card
export function hideEncodingInfo() {
    const { encodingInfoCard } = getElements();
    if (encodingInfoCard) {
        encodingInfoCard.classList.add('hidden');
    }
}

// grab metadata values from the editor form
export function getMetadataFromEditor() {
    const title = document.getElementById('metaTitle')?.value?.trim();
    const artist = document.getElementById('metaArtist')?.value?.trim();
    const album = document.getElementById('metaAlbum')?.value?.trim();
    const yearStr = document.getElementById('metaYear')?.value?.trim();
    const genre = document.getElementById('metaGenre')?.value?.trim();
    const bpmStr = document.getElementById('metaBpm')?.value?.trim();
    const key = document.getElementById('metaKey')?.value?.trim();
    const trackStr = document.getElementById('metaTrack')?.value?.trim();
    const comment = document.getElementById('metaComment')?.value?.trim();
    
    // only include stuff that has values
    const meta = {};
    if (title) meta.title = title;
    if (artist) meta.artist = artist;
    if (album) meta.album = album;
    if (yearStr) meta.year = parseInt(yearStr, 10);
    if (genre) meta.genre = genre;
    if (bpmStr) meta.bpm = parseInt(bpmStr, 10);
    if (key) meta.key = key;
    if (trackStr) meta.track_number = parseInt(trackStr, 10);
    if (comment) meta.comments = [{ text: comment }];
    
    return Object.keys(meta).length > 0 ? meta : null;
}

// nuke all the metadata fields
export function clearMetadataEditor() {
    document.getElementById('metaTitle').value = '';
    document.getElementById('metaArtist').value = '';
    document.getElementById('metaAlbum').value = '';
    document.getElementById('metaYear').value = '';
    document.getElementById('metaGenre').value = '';
    document.getElementById('metaBpm').value = '';
    document.getElementById('metaKey').value = '';
    document.getElementById('metaTrack').value = '';
    document.getElementById('metaComment').value = '';
}

// show or hide the metadata editor section
export function toggleMetadataEditor() {
    const content = document.getElementById('metadataEditorContent');
    const chevron = document.getElementById('metadataEditorChevron');
    
    if (content.classList.contains('hidden')) {
        content.classList.remove('hidden');
        chevron.style.transform = 'rotate(180deg)';
    } else {
        content.classList.add('hidden');
        chevron.style.transform = 'rotate(0deg)';
    }
}

// lil helpers
function metaRow(label, value) {
    return `<div class="meta-row"><span class="meta-label">${label}</span><span class="meta-value">${value}</span></div>`;
}

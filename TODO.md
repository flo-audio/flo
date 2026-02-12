`# Todo

## Analysis Functions

- [x] Add spectrum fingerprint analysis function to libflo
- [x] Add EBU R128 loudness metrics to libflo (integrated loudness LUFS, loudness range LU, true peak dBTP)
- [x] Add waveform peaks extraction to libflo/reflo (peaks per second, channels)
- [x] Auto-add these to metadata on encode
- [x] Export standalone analysis functions to WASM/JS interface for on-demand use
  - `extract_spectral_fingerprint_wasm(samples, channels, sample_rate, fft_size, hop_size)`
  - `compute_loudness_metrics(samples, channels, sample_rate)`
  - `extract_waveform_peaks_wasm(samples, channels, sample_rate, peaks_per_second)`

## Demo Improvements

- [x] Add audio analysis panel with EBU R128 loudness visualization
- [x] Add frequency spectrum bar chart
- [x] Loudness meter with color-coded zones
- [x] Live visualizer using Web Audio AnalyserNode for real-time FFT during playback
- [x] Display File & Encoding Info (encoder version, encoding time, source format, etc.)

## Seeking & Playback

- [ ] Add TOC-based seeking for large files (without fully decoding)
  - `get_toc(flo_data)` - Return TOC entries with timestamp_ms → byte_offset mapping
  - `decode_frame_at(flo_data, frame_index)` - Decode specific frame by index
  - `seek_to_time(flo_data, time_ms)` - Find frame and decode from position
- [ ] Streaming playback with on-demand frame decoding

## QoL

- [ ] Add CLI tool for file inspection (info, metadata, analysis)
- [ ] Add streaming encode support (currently only decode streams)
- [ ] Add file comparison view (compare original vs encoded)

# Bugs

- [ ] Investigate: some example .flo files may have invalid total_frames (audio_lossless.flo shows total_frames=1)
- [X] fix bug with lossy encoded files having broken duration (1:50:00 ish)

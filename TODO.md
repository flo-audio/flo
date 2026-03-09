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

- [x] Add TOC-based seeking for large files (without fully decoding)
  - `get_toc(flo_data)` - Return TOC entries with timestamp_ms → byte_offset mapping
  - `decode_frame_at(flo_data, frame_index)` - Decode specific frame by index
  - `seek_to_time(flo_data, time_ms)` - Find frame and decode from position
- [x] Streaming playback with on-demand frame decoding

## QoL

- [x] Add CLI tool for file inspection (info, metadata, analysis)
- [ ] Add bulk converter to reflo + web demo (possibly separate page)
- [x] Add streaming encode support (currently only decode streams) (To clarify, it does exist but needs to be implemented into the web demo and exported to WASM)
- [ ] Add streaming encode to web demo
- [ ] Test parity between Rust and Jest and more reflo tests
- [ ] Add file comparison view (compare original vs encoded)

# Bugs

- [x] Investigate: some example .flo files may have invalid total_frames (audio_lossless.flo shows total_frames=1)
  - Changed `total_frames` to `total_samples` and it stores the actual sample count
  - Made `length_ms` always written to metadata during encode
  - Reader now uses `length_ms` for duration (with fallback to calculation for older files)
- [x] fix bug with lossy encoded files having broken duration (1:50:00 ish)

# TODO

- [x] Add spectrum fingerprint analysis function to libflo (WASM export as well)
- [ ] Add EBU R128 loudness metrics to libflo (integrated loudness LUFS, loudness range LU, true peak dBTP)
- [x] Add waveform peaks extraction to libflo/reflo (peaks per second, channels)
- [x] Export new analysis functions from libflo to WASM/JS interface
- [x] auto add these to metadata on encode

- [ ] fix bug with lossy encoded files having broken duration (1:50:00 ish)
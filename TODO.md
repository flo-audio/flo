# Todo

- [ ] Add spectrum fingerprint analysis function to libflo (WASM export as well)
- [ ] Add EBU R128 loudness metrics to libflo (integrated loudness LUFS, loudness range LU, true peak dBTP)
- [ ] Add waveform peaks extraction to libflo/reflo (peaks per second, channels)
- [ ] Export new analysis functions from libflo to WASM/JS interface
- [ ] auto add these to metadata on encode

# Bugs

- [ ] fix bug with lossy encoded files having broken duration (1:50:00 ish)
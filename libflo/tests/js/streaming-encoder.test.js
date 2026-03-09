/**
 * Jest tests for WasmStreamingEncoder WASM API
 * 
 * Tests for:
 * - WasmStreamingEncoder creation and initialization
 * - Sample pushing and frame extraction
 * - Compression level configuration
 * - Finalization and file generation
 * - Integration with WasmStreamingDecoder
 */

import * as libflo from '../pkg/libflo_audio.js';

describe('WasmStreamingEncoder', () => {
  describe('creation and initialization', () => {
    test('should create encoder with default parameters', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 2, 16);
      expect(encoder).toBeDefined();
      expect(encoder.pending_samples()).toBe(0);
      expect(encoder.pending_frames()).toBe(0);
    });

    test('should create encoder with various sample rates', () => {
      const sampleRates = [8000, 16000, 22050, 44100, 48000, 96000];
      
      for (const sr of sampleRates) {
        const encoder = new libflo.WasmStreamingEncoder(sr, 2, 16);
        expect(encoder).toBeDefined();
        expect(encoder.pending_samples()).toBe(0);
        expect(encoder.pending_frames()).toBe(0);
      }
    });

    test('should create encoder with various channel counts', () => {
      const encoder1 = new libflo.WasmStreamingEncoder(44100, 1, 16);
      expect(encoder1).toBeDefined();
      
      const encoder2 = new libflo.WasmStreamingEncoder(44100, 2, 16);
      expect(encoder2).toBeDefined();
    });

    test('should create encoder with various bit depths', () => {
      const bitDepths = [16, 24, 32];
      
      for (const bd of bitDepths) {
        const encoder = new libflo.WasmStreamingEncoder(44100, 2, bd);
        expect(encoder).toBeDefined();
      }
    });
  });

  describe('sample pushing and frame extraction', () => {
    test('should accept audio samples', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      // Fill with sine wave
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      expect(() => encoder.push_samples(samples)).not.toThrow();
    });

    test('should generate frames after pushing samples', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      expect(encoder.pending_frames()).toBeGreaterThan(0);
    });

    test('should extract frames via next_frame()', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      
      let frameCount = 0;
      let frame = encoder.next_frame();
      while (frame !== null) {
        expect(frame).toHaveProperty('index');
        expect(frame).toHaveProperty('timestamp_ms');
        expect(frame).toHaveProperty('data');
        expect(frame).toHaveProperty('samples');
        expect(frame.data instanceof Uint8Array).toBe(true);
        frameCount++;
        frame = encoder.next_frame();
      }
      
      expect(frameCount).toBeGreaterThan(0);
    });

    test('should track frame indices correctly', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100 * 2); // 2 seconds
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      
      let lastIndex = -1;
      let frame = encoder.next_frame();
      while (frame !== null) {
        expect(frame.index).toBe(lastIndex + 1);
        lastIndex = frame.index;
        frame = encoder.next_frame();
      }
      
      expect(lastIndex).toBeGreaterThanOrEqual(1);
    });
  });

  describe('compression levels', () => {
    test('should accept compression levels 0-9', () => {
      for (let level = 0; level <= 9; level++) {
        const encoder = new libflo.WasmStreamingEncoder(44100, 2, 16);
        const compressed = encoder.with_compression(level);
        expect(compressed).toBeDefined();
      }
    });

    test('should apply compression level to encoding', () => {
      const samples = new Float32Array(44100);
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }

      // Low compression (faster)
      const encoder0 = new libflo.WasmStreamingEncoder(44100, 1, 16);
      encoder0.with_compression(0);
      encoder0.push_samples(samples);
      encoder0.flush();
      const file0 = encoder0.finalize(null);
      
      // High compression (slower)
      const encoder9 = new libflo.WasmStreamingEncoder(44100, 1, 16);
      encoder9.with_compression(9);
      encoder9.push_samples(samples);
      encoder9.flush();
      const file9 = encoder9.finalize(null);
      
      // Higher compression should generally produce smaller or equal files
      expect(file9.length).toBeLessThanOrEqual(file0.length * 1.1); // Allow 10% margin
    });
  });

  describe('flush and finalization', () => {
    test('should flush partial frames', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      
      // Push less than a full frame (22050 samples = 0.5 seconds)
      const samples = new Float32Array(22050);
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      expect(() => encoder.flush()).not.toThrow();
      expect(encoder.pending_frames()).toBeGreaterThan(0);
    });

    test('should finalize without metadata', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      encoder.flush();
      const file = encoder.finalize(null);
      
      expect(file).toBeInstanceOf(Uint8Array);
      expect(file.length).toBeGreaterThan(0);
      
      // Check flo magic bytes
      expect(file[0]).toBe(0x46); // 'F'
      expect(file[1]).toBe(0x4c); // 'L'
      expect(file[2]).toBe(0x4f); // 'O'
      expect(file[3]).toBe(0x21); // '!'
    });

    test('should finalize with metadata', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 2, 16);
      const samples = new Float32Array(44100 * 2);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      encoder.flush();
      
      // Create simple metadata (MessagePack empty map)
      const metadata = new Uint8Array([0xdc, 0x00, 0x00]);
      const file = encoder.finalize(metadata);
      
      expect(file).toBeInstanceOf(Uint8Array);
      expect(file.length).toBeGreaterThan(0);
      expect(file[0]).toBe(0x46);
    });
  });

  describe('chunked encoding', () => {
    test('should handle samples pushed in chunks', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      
      // Generate 2 seconds total
      const totalSamples = 88200;
      const chunkSize = 4410; // ~100ms chunks
      
      for (let i = 0; i < totalSamples; i += chunkSize) {
        const chunk = new Float32Array(Math.min(chunkSize, totalSamples - i));
        for (let j = 0; j < chunk.length; j++) {
          const idx = i + j;
          chunk[j] = Math.sin((idx * 0.01) * 2 * Math.PI);
        }
        encoder.push_samples(chunk);
      }
      
      encoder.flush();
      const file = encoder.finalize(null);
      
      expect(file.length).toBeGreaterThan(0);
      expect(file[0]).toBe(0x46);
    });

    test('should maintain state across multiple push calls', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      
      let totalFrames = 0;
      
      // Push in 3 batches
      for (let batch = 0; batch < 3; batch++) {
        const samples = new Float32Array(44100);
        for (let i = 0; i < samples.length; i++) {
          samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
        }
        encoder.push_samples(samples);
        
        // Count frames available after each push
        let frame = encoder.next_frame();
        while (frame !== null) {
          totalFrames++;
          frame = encoder.next_frame();
        }
      }
      
      encoder.flush();
      let frame = encoder.next_frame();
      while (frame !== null) {
        totalFrames++;
        frame = encoder.next_frame();
      }
      
      expect(totalFrames).toBeGreaterThan(0);
    });
  });

  describe('stereo encoding', () => {
    test('should encode stereo audio correctly', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 2, 16);
      
      // Create interleaved stereo (L, R, L, R, ...)
      const samples = new Float32Array(44100 * 2);
      for (let i = 0; i < 44100; i++) {
        samples[i * 2] = Math.sin((i * 0.01) * 2 * Math.PI);     // Left
        samples[i * 2 + 1] = Math.cos((i * 0.015) * 2 * Math.PI); // Right
      }
      
      encoder.push_samples(samples);
      encoder.flush();
      const file = encoder.finalize(null);
      
      expect(file.length).toBeGreaterThan(0);
      expect(file[0]).toBe(0x46);
    });
  });

  describe('edge cases', () => {
    test('should handle silence (all zeros)', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const silence = new Float32Array(44100);
      
      encoder.push_samples(silence);
      encoder.flush();
      const file = encoder.finalize(null);
      
      expect(file.length).toBeGreaterThan(0);
      expect(file[0]).toBe(0x46);
    });

    test('should handle very quiet audio', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI) * 0.00001;
      }
      
      encoder.push_samples(samples);
      encoder.flush();
      const file = encoder.finalize(null);
      
      expect(file.length).toBeGreaterThan(0);
    });

    test('should handle clipped audio (out of [-1, 1] range)', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI) * 2.0; // Clipped
      }
      
      expect(() => {
        encoder.push_samples(samples);
        encoder.flush();
        encoder.finalize(null);
      }).not.toThrow();
    });

    test('should handle empty finalization (no samples)', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      
      encoder.flush();
      const file = encoder.finalize(null);
      
      // Should still produce a valid (empty) file
      expect(file.length).toBeGreaterThan(0);
    });
  });

  describe('integration with WasmStreamingDecoder', () => {
    test('should produce files decodable by WasmStreamingDecoder', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const samples = new Float32Array(44100);
      
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      encoder.flush();
      const file = encoder.finalize(null);
      
      // Try to decode
      const decoder = new libflo.WasmStreamingDecoder();
      expect(() => decoder.feed(file)).not.toThrow();
      expect(decoder.is_ready()).toBe(true);
    });

    test('should decode to approximately original samples', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      const originalSamples = new Float32Array(44100);
      
      for (let i = 0; i < originalSamples.length; i++) {
        originalSamples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(originalSamples);
      encoder.flush();
      const file = encoder.finalize(null);
      
      // Decode
      const decoder = new libflo.WasmStreamingDecoder();
      decoder.feed(file);
      const decodedSamples = decoder.decode_available();
      
      expect(decodedSamples.length).toBe(originalSamples.length);
      
      // Check that samples are close (lossless should be very close)
      let maxDiff = 0;
      for (let i = 0; i < decodedSamples.length; i++) {
        const diff = Math.abs(originalSamples[i] - decodedSamples[i]);
        maxDiff = Math.max(maxDiff, diff);
      }
      
      expect(maxDiff).toBeLessThan(0.001);
    });

    test('should handle frame-by-frame decoding', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 2, 16);
      const samples = new Float32Array(44100 * 2);
      
      for (let i = 0; i < 44100; i++) {
        samples[i * 2] = Math.sin((i * 0.01) * 2 * Math.PI);
        samples[i * 2 + 1] = Math.cos((i * 0.015) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      encoder.flush();
      const file = encoder.finalize(null);
      
      const decoder = new libflo.WasmStreamingDecoder();
      decoder.feed(file);
      
      let frameCount = 0;
      let frame = decoder.next_frame();
      while (frame !== null) {
        expect(frame instanceof Float32Array).toBe(true);
        expect(frame.length).toBeGreaterThan(0);
        frameCount++;
        frame = decoder.next_frame();
      }
      
      expect(frameCount).toBeGreaterThan(0);
    });
  });

  describe('pending state management', () => {
    test('should report pending samples correctly', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      
      expect(encoder.pending_samples()).toBe(0);
      
      const halfSecond = new Float32Array(22050);
      for (let i = 0; i < halfSecond.length; i++) {
        halfSecond[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(halfSecond);
      expect(encoder.pending_samples()).toBeGreaterThan(0);
      
      encoder.flush();
      expect(encoder.pending_samples()).toBe(0);
    });

    test('should report pending frames correctly', () => {
      const encoder = new libflo.WasmStreamingEncoder(44100, 1, 16);
      
      expect(encoder.pending_frames()).toBe(0);
      
      const samples = new Float32Array(44100);
      for (let i = 0; i < samples.length; i++) {
        samples[i] = Math.sin((i * 0.01) * 2 * Math.PI);
      }
      
      encoder.push_samples(samples);
      expect(encoder.pending_frames()).toBeGreaterThan(0);
      
      const initialCount = encoder.pending_frames();
      encoder.next_frame();
      expect(encoder.pending_frames()).toBeLessThan(initialCount);
    });
  });
});

meta:
  id: flo_audio
  title: flo™ Audio Format
  file-extension: flo
  endian: le
  license: Apache-2.0
  
doc: |
  flo™ (Fast Layered Object): A modern dual-mode audio format
  
  Design principles:
  - HEAD contains all chunk offsets (single source of truth)
  - Fixed-duration frames (1 second) with variable size
  - Dual-mode: Lossless (ALPC) or Lossy (Transform/MDCT)
  - MessagePack metadata for flexibility
  
  Encoding modes:
  - Lossless (ALPC): Adaptive Linear Predictive Coding with Rice entropy coding
    - Achieves ~2-3x compression
    - Perfect bit-for-bit reconstruction
    - Frame types 1-12 indicate LPC order
  
  - Lossy (Transform): MDCT-based psychoacoustic compression
    - Achieves ~10-30x compression depending on quality
    - Uses 2048-sample MDCT blocks with 50% overlap
    - Psychoacoustic masking discards inaudible frequencies
    - Frame type 253 indicates transform mode
    - Quality levels: Low(0), Medium(1), High(2), VeryHigh(3), Transparent(4)

seq:
  - id: header
    type: flo_header
    doc: Main header containing format info and chunk offsets
  
  - id: toc
    type: toc_chunk
    size: header.toc_size
    doc: Table of contents with seek points
  
  - id: data
    type: data_chunk
    size: header.data_size
    doc: Compressed audio frames
  
  - id: extra
    type: extra_chunk
    size: header.extra_size
    doc: Reserved for future extensions
  
  - id: meta
    type: meta_chunk
    size: header.meta_size
    doc: MessagePack metadata (artist, album, etc)

types:
  flo_header:
    seq:
      - id: magic
        contents: [0x46, 0x4C, 0x4F, 0x21]  # "FLO!"
        doc: Magic number identifier
      
      - id: version_major
        type: u1
        doc: Format major version (currently 1)
      
      - id: version_minor
        type: u1
        doc: Format minor version (currently 2)
      
      - id: flags
        type: u2
        doc: |
          Bit flags:
          Bit 0: Lossy mode enabled (0=lossless, 1=lossy)
          Bits 8-11: Lossy quality level (0=Low, 1=Medium, 2=High, 3=VeryHigh, 4=Transparent)
          Other bits: Reserved
      
      - id: sample_rate
        type: u4
        doc: Sample rate in Hz (e.g., 44100, 48000)
      
      - id: channels
        type: u1
        doc: Number of audio channels (1=mono, 2=stereo)
      
      - id: bit_depth
        type: u1
        doc: Bits per sample (16, 24, or 32)
      
      - id: total_frames
        type: u8
        doc: Duration in seconds (number of 1-second frames)
      
      - id: compression_level
        type: u1
        doc: Global compression hint (0-9, affects ALPC order selection)
      
      - id: reserved
        size: 3
        doc: Reserved for future use (must be 0)

      # Integrity Check
      - id: data_crc32
        type: u4
        doc: CRC32 checksum of the DATA chunk integrity

      # Sizes
      - id: header_size
        type: u8
        doc: Size of this header in bytes (excludes magic)
      
      - id: toc_size
        type: u8
        doc: Size of TOC chunk in bytes
      
      - id: data_size
        type: u8
        doc: Size of DATA chunk in bytes
      
      - id: extra_size
        type: u8
        doc: Size of EXTRA chunk in bytes
      
      - id: meta_size
        type: u8
        doc: Size of META chunk in bytes
    
    instances:
      is_lossy:
        value: (flags & 0x01) != 0
        doc: True if lossy compression is enabled
      
      lossy_quality:
        value: (flags >> 8) & 0x0F
        doc: Lossy quality level (0=Low, 1=Medium, 2=High, 3=VeryHigh, 4=Transparent)

  toc_chunk:
    seq:
      - id: num_entries
        type: u4
        doc: Number of seek points
      
      - id: entries
        type: toc_entry
        repeat: expr
        repeat-expr: num_entries

  toc_entry:
    seq:
      - id: frame_index
        type: u4
        doc: Frame number (0-based)
      
      - id: byte_offset
        type: u8
        doc: Byte offset from start of DATA chunk
      
      - id: frame_size
        type: u4
        doc: Size of this frame in bytes
      
      - id: timestamp_ms
        type: u4
        doc: Timestamp in milliseconds

  data_chunk:
    seq:
      - id: frames
        type: audio_frame
        repeat: eos
        doc: Sequence of 1-second audio frames

  audio_frame:
    seq:
      - id: frame_header
        type: frame_header
      
      - id: channels
        type: channel_wrapper
        repeat: expr
        repeat-expr: _root.header.channels

  channel_wrapper:
    doc: Each channel is prefixed with its size for variable-length support
    seq:
      - id: channel_size
        type: u4
        doc: Size of this channel's data in bytes
      
      - id: channel_data
        type: channel_data
        size: channel_size

  frame_header:
    seq:
      - id: frame_type
        type: u1
        enum: frame_type_enum
        doc: |
          0 = Silence
          1-12 = ALPC with order N (lossless)
          253 = Transform (MDCT-based lossy)
          254 = Raw PCM
          255 = Reserved
      
      - id: frame_samples
        type: u4
        doc: Number of samples in this frame (usually sample_rate)
      
      - id: flags
        type: u1
        doc: Per-frame flags (reserved)
    
    enums:
      frame_type_enum:
        0: silence
        1: alpc_order_1
        2: alpc_order_2
        3: alpc_order_3
        4: alpc_order_4
        5: alpc_order_5
        6: alpc_order_6
        7: alpc_order_7
        8: alpc_order_8
        9: alpc_order_9
        10: alpc_order_10
        11: alpc_order_11
        12: alpc_order_12
        253: transform
        254: raw_pcm
        255: reserved

  channel_data:
    doc: |
      Channel data format depends on frame type.
      
      ALPC (types 1-12): LPC-based lossless compression
        - coeff_count, predictor_coeffs, shift_bits, residual_encoding, rice_parameter, residuals
      
      Transform (type 253): MDCT-based lossy compression
        - block_size (1 byte): 0=Long(2048), 1=Short(256), 2=Start, 3=Stop
        - scale_factors (25 bands * 2 bytes): Log-scale u16 per Bark band
        - coefficient_length (4 bytes): Size of sparse coefficient data
        - coefficients: Sparse RLE encoded i16 MDCT coefficients
      
      Raw (type 254): Uncompressed PCM samples
      
      Silence (type 0): No data (frame represents silence)
    seq:
      # ALPC fields (frame types 1-12)
      - id: coeff_count
        type: u1
        if: _parent._parent.frame_header.frame_type.to_i >= 1 and _parent._parent.frame_header.frame_type.to_i <= 12
        doc: Actual number of LPC coefficients (may differ from frame_type due to stability fallback)
      
      - id: predictor_coeffs
        type: s4
        repeat: expr
        repeat-expr: coeff_count
        if: _parent._parent.frame_header.frame_type.to_i >= 1 and _parent._parent.frame_header.frame_type.to_i <= 12
        doc: LPC coefficients (quantized as i32)
      
      - id: shift_bits
        type: u1
        if: _parent._parent.frame_header.frame_type.to_i >= 1 and _parent._parent.frame_header.frame_type.to_i <= 12
        doc: Bit shift for coefficient dequantization
      
      - id: residual_encoding
        type: u1
        if: _parent._parent.frame_header.frame_type.to_i >= 1 and _parent._parent.frame_header.frame_type.to_i <= 12
        doc: |
          Residual encoding method:
          0 = Rice coding (parameter in next byte)
          1 = Golomb coding
          2 = Raw residuals
      
      - id: rice_parameter
        type: u1
        if: _parent._parent.frame_header.frame_type.to_i >= 1 and _parent._parent.frame_header.frame_type.to_i <= 12 and residual_encoding == 0
        doc: Rice coding parameter (k value)
      
      - id: residuals
        size-eos: true
        doc: |
          For ALPC: Rice/Golomb coded or raw residuals
          For Transform: MDCT coefficient data (block_size + scale_factors + sparse coefficients)
          For Raw: Uncompressed PCM samples

  extra_chunk:
    seq:
      - id: data
        size-eos: true
        doc: Reserved for future use

  meta_chunk:
    seq:
      - id: msgpack_data
        size-eos: true
        doc: |
          MessagePack encoded metadata (FloMetadata struct)
          
          IDENTIFICATION (ID3v2.4 compatible):
            title, subtitle, content_group, album, original_album, set_subtitle
            track_number, track_total, disc_number, disc_total, isrc
          
          INVOLVED PERSONS:
            artist, album_artist, conductor, remixer, original_artist
            composer, lyricist, original_lyricist, encoded_by
            involved_people: [(role, name), ...]
            musician_credits: [(instrument, name), ...]
          
          PROPERTIES:
            genre, mood, bpm (u32), key, language, length_ms (u64)
          
          DATES/TIMES:
            year, recording_time, release_time, original_release_time
            encoding_time, tagging_time
          
          RIGHTS/LICENSE:
            copyright, produced_notice, publisher, file_owner
            radio_station, radio_station_owner
          
          SORT ORDER:
            album_sort, artist_sort, title_sort
          
          URLS:
            url_commercial, url_copyright, url_audio_file, url_artist
            url_audio_source, url_radio_station, url_payment, url_publisher
            user_urls: [{description, url}, ...]
          
          COMPLEX FRAMES:
            comments: [{language?, description?, text}, ...]
            lyrics: [{language?, description?, text}, ...]  # USLT
            synced_lyrics: [{language?, content_type, description?, lines}, ...]  # SYLT
            pictures: [{mime_type, picture_type, description?, data}, ...]  # APIC
            user_text: [{description, value}, ...]  # TXXX
            play_count, popularimeter: {email?, rating, play_count?}  # PCNT, POPM
          
          SYNCED LYRICS (content_type):
            other, lyrics, text_transcription, part_name, events, chord, trivia, webpage_url, image_url
          
          PICTURE TYPES:
            other, file_icon, other_file_icon, cover_front, cover_back
            leaflet_page, media, lead_artist, artist, conductor, band
            composer, lyricist, recording_location, during_recording
            during_performance, video_screen_capture, bright_coloured_fish
            illustration, band_logo, publisher_logo
          
          VISUALIZATION (flo™-unique):
            waveform_data: {peaks_per_second, peaks: [f32], channels}
            spectrum_fingerprint: bytes
          
          TIMING & ANALYSIS (flo™-unique):
            bpm_map: [{timestamp_ms, bpm}, ...]
            key_changes: [{timestamp_ms, key}, ...]
            loudness_profile: [{timestamp_ms, lufs}, ...]
            integrated_loudness_lufs, loudness_range_lu, true_peak_dbtp (f32)
            section_markers: [{timestamp_ms, section_type, label?}, ...]
          
          SECTION TYPES:
            intro, verse, pre_chorus, chorus, post_chorus, bridge
            breakdown, drop, buildup, solo, instrumental, outro, silence, other
          
          CREATOR INFO (flo™-unique):
            creator_notes: [{timestamp_ms?, text}, ...]
            collaboration_credits: [{role, name, timestamp_ms?}, ...]
            remix_chain: [{title, artist, year?, isrc?, relationship}, ...]
          
          COVERS (flo™-unique):
            animated_cover: {mime_type, data, duration_ms?, loop_count?}
            cover_variants: [{variant_type, mime_type, data, description?}, ...]
            artist_signature: Picture (same as pictures entry)
          
          COVER VARIANT TYPES:
            standard, explicit, clean, remix, deluxe, limited, vinyl, cassette, digital, other
          
          flo™-SPECIFIC:
            flo_encoder_version, source_format
            custom: {key: value, ...}

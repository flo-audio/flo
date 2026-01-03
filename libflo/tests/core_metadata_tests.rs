//! Metadata tests for libflo

use libflo_audio::{
    AnimatedCover, BpmChange, CollaborationCredit, CoverVariant, CoverVariantType, CreatorNote,
    FloMetadata, KeyChange, LoudnessPoint, Picture, PictureType, RemixChainEntry, SectionMarker,
    SectionType, SyncedLyrics, SyncedLyricsContentType, SyncedLyricsLine, WaveformData,
};

// ============================================================================
// Basic Metadata Tests
// ============================================================================

#[test]
fn test_metadata_empty() {
    let meta = FloMetadata::new();
    assert!(meta.is_empty());

    let packed = meta.to_msgpack().unwrap();
    assert!(packed.len() < 50, "Empty metadata should be small");
}

#[test]
fn test_metadata_basic_fields() {
    let meta = FloMetadata::with_basic(
        Some("My Song".to_string()),
        Some("My Artist".to_string()),
        Some("My Album".to_string()),
    );

    assert_eq!(meta.title, Some("My Song".to_string()));
    assert_eq!(meta.artist, Some("My Artist".to_string()));
    assert_eq!(meta.album, Some("My Album".to_string()));
    assert!(!meta.is_empty());
}

#[test]
fn test_metadata_roundtrip() {
    let mut meta = FloMetadata::new();
    meta.title = Some("Test Song".to_string());
    meta.artist = Some("Test Artist".to_string());
    meta.album = Some("Test Album".to_string());
    meta.year = Some(2024);
    meta.track_number = Some(1);
    meta.track_total = Some(12);
    meta.genre = Some("Electronic".to_string());
    meta.bpm = Some(128);
    meta.key = Some("Am".to_string());
    meta.mood = Some("Energetic".to_string());

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.title, meta.title);
    assert_eq!(unpacked.artist, meta.artist);
    assert_eq!(unpacked.album, meta.album);
    assert_eq!(unpacked.year, meta.year);
    assert_eq!(unpacked.track_number, meta.track_number);
    assert_eq!(unpacked.genre, meta.genre);
    assert_eq!(unpacked.bpm, meta.bpm);
    assert_eq!(unpacked.key, meta.key);
}

// ============================================================================
// Picture Tests
// ============================================================================

#[test]
fn test_picture_front_cover() {
    let mut meta = FloMetadata::new();
    // Fake JPEG header
    meta.add_picture(
        "image/jpeg",
        PictureType::CoverFront,
        vec![0xFF, 0xD8, 0xFF, 0xE0],
    );

    assert!(!meta.is_empty());
    assert!(meta.front_cover().is_some());
    assert_eq!(meta.front_cover().unwrap().mime_type, "image/jpeg");
}

#[test]
fn test_multiple_pictures() {
    let mut meta = FloMetadata::new();
    meta.add_picture("image/jpeg", PictureType::CoverFront, vec![1, 2, 3]);
    meta.add_picture("image/png", PictureType::CoverBack, vec![4, 5, 6]);
    meta.add_picture("image/jpeg", PictureType::Artist, vec![7, 8, 9]);

    assert_eq!(meta.pictures.len(), 3);
    assert!(meta.front_cover().is_some());
    assert!(meta.any_picture().is_some());
}

#[test]
fn test_picture_types() {
    // Test the fish! 🐟
    let mut meta = FloMetadata::new();
    meta.add_picture(
        "image/png",
        PictureType::BrightColouredFish,
        vec![0x89, 0x50, 0x4E, 0x47],
    );

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(
        unpacked.pictures[0].picture_type,
        PictureType::BrightColouredFish
    );
}

// ============================================================================
// Comments & Lyrics Tests
// ============================================================================

#[test]
fn test_comments() {
    let mut meta = FloMetadata::new();
    meta.add_comment("Great song!", Some("eng"));
    meta.add_comment("素晴らしい曲!", Some("jpn"));

    assert_eq!(meta.comments.len(), 2);
    assert_eq!(meta.comments[0].text, "Great song!");
    assert_eq!(meta.comments[0].language, Some("eng".to_string()));
}

#[test]
fn test_unsync_lyrics() {
    let mut meta = FloMetadata::new();
    meta.add_lyrics("Verse 1\nChorus\nVerse 2", Some("eng"));

    assert_eq!(meta.lyrics.len(), 1);
    assert!(meta.lyrics[0].text.contains("Chorus"));
}

#[test]
fn test_synced_lyrics() {
    let mut meta = FloMetadata::new();

    let synced = SyncedLyrics {
        language: Some("eng".to_string()),
        content_type: SyncedLyricsContentType::Lyrics,
        description: Some("Main lyrics".to_string()),
        lines: vec![
            SyncedLyricsLine {
                timestamp_ms: 0,
                text: "First line".to_string(),
            },
            SyncedLyricsLine {
                timestamp_ms: 3000,
                text: "Second line".to_string(),
            },
            SyncedLyricsLine {
                timestamp_ms: 6000,
                text: "Third line".to_string(),
            },
        ],
    };

    meta.synced_lyrics.push(synced);

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.synced_lyrics.len(), 1);
    assert_eq!(unpacked.synced_lyrics[0].lines.len(), 3);
    assert_eq!(unpacked.synced_lyrics[0].lines[1].timestamp_ms, 3000);
    assert_eq!(unpacked.synced_lyrics[0].lines[1].text, "Second line");
}

// ============================================================================
// flo-Unique Features Tests
// ============================================================================

#[test]
fn test_waveform_data() {
    let mut meta = FloMetadata::new();

    meta.waveform_data = Some(WaveformData {
        peaks_per_second: 10,
        peaks: vec![0.1, 0.5, 0.8, 0.3, 0.6, 0.9, 0.4, 0.2, 0.7, 0.5],
        channels: 1,
    });

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert!(unpacked.waveform_data.is_some());
    let waveform = unpacked.waveform_data.unwrap();
    assert_eq!(waveform.peaks_per_second, 10);
    assert_eq!(waveform.peaks.len(), 10);
}

#[test]
fn test_section_markers() {
    let mut meta = FloMetadata::new();

    meta.section_markers = vec![
        SectionMarker {
            timestamp_ms: 0,
            section_type: SectionType::Intro,
            label: Some("Intro".to_string()),
        },
        SectionMarker {
            timestamp_ms: 15000,
            section_type: SectionType::Verse,
            label: Some("Verse 1".to_string()),
        },
        SectionMarker {
            timestamp_ms: 45000,
            section_type: SectionType::Chorus,
            label: None,
        },
        SectionMarker {
            timestamp_ms: 75000,
            section_type: SectionType::Bridge,
            label: Some("Bridge".to_string()),
        },
        SectionMarker {
            timestamp_ms: 90000,
            section_type: SectionType::Outro,
            label: None,
        },
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.section_markers.len(), 5);
    assert_eq!(
        unpacked.section_markers[2].section_type,
        SectionType::Chorus
    );
}

#[test]
fn test_bpm_map() {
    let mut meta = FloMetadata::new();
    meta.bpm = Some(120); // Base BPM

    meta.bpm_map = vec![
        BpmChange {
            timestamp_ms: 0,
            bpm: 120.0,
        },
        BpmChange {
            timestamp_ms: 60000,
            bpm: 140.0,
        }, // Speed up at 1:00
        BpmChange {
            timestamp_ms: 120000,
            bpm: 120.0,
        }, // Back to normal at 2:00
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.bpm_map.len(), 3);
    assert_eq!(unpacked.bpm_map[1].bpm, 140.0);
}

#[test]
fn test_key_changes() {
    let mut meta = FloMetadata::new();
    meta.key = Some("Am".to_string()); // Starting key

    meta.key_changes = vec![
        KeyChange {
            timestamp_ms: 0,
            key: "Am".to_string(),
        },
        KeyChange {
            timestamp_ms: 90000,
            key: "Cm".to_string(),
        }, // Modulation at 1:30
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.key_changes.len(), 2);
    assert_eq!(unpacked.key_changes[1].key, "Cm");
}

#[test]
fn test_loudness_profile() {
    let mut meta = FloMetadata::new();

    // Frame-by-frame loudness (one per second for this test)
    meta.loudness_profile = vec![
        LoudnessPoint {
            timestamp_ms: 0,
            lufs: -14.0,
        },
        LoudnessPoint {
            timestamp_ms: 1000,
            lufs: -12.0,
        },
        LoudnessPoint {
            timestamp_ms: 2000,
            lufs: -8.0,
        },
        LoudnessPoint {
            timestamp_ms: 3000,
            lufs: -10.0,
        },
    ];
    meta.integrated_loudness_lufs = Some(-11.0);
    meta.loudness_range_lu = Some(6.0);
    meta.true_peak_dbtp = Some(-1.0);

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.loudness_profile.len(), 4);
    assert_eq!(unpacked.integrated_loudness_lufs, Some(-11.0));
}

#[test]
fn test_creator_notes() {
    let mut meta = FloMetadata::new();

    meta.creator_notes = vec![
        CreatorNote {
            timestamp_ms: Some(30000),
            text: "This synth was recorded through a vintage Moog".to_string(),
        },
        CreatorNote {
            timestamp_ms: None,
            text: "Mixed in Abbey Road Studios".to_string(),
        },
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.creator_notes.len(), 2);
}

#[test]
fn test_collaboration_credits() {
    let mut meta = FloMetadata::new();

    meta.collaboration_credits = vec![
        CollaborationCredit {
            role: "Lead Vocals".to_string(),
            name: "Jane Doe".to_string(),
            timestamp_ms: None,
        },
        CollaborationCredit {
            role: "Guitar Solo".to_string(),
            name: "John Smith".to_string(),
            timestamp_ms: Some(120000), // Solo at 2:00
        },
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.collaboration_credits.len(), 2);
    assert_eq!(unpacked.collaboration_credits[1].role, "Guitar Solo");
}

#[test]
fn test_remix_chain() {
    let mut meta = FloMetadata::new();

    meta.remix_chain = vec![
        RemixChainEntry {
            title: "Original Song".to_string(),
            artist: "Original Artist".to_string(),
            year: Some(2020),
            isrc: Some("USRC12000001".to_string()),
            relationship: "original".to_string(),
        },
        RemixChainEntry {
            title: "First Remix".to_string(),
            artist: "DJ Remix".to_string(),
            year: Some(2022),
            isrc: None,
            relationship: "remix".to_string(),
        },
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.remix_chain.len(), 2);
}

#[test]
fn test_animated_cover() {
    let mut meta = FloMetadata::new();

    // Fake GIF data
    meta.animated_cover = Some(AnimatedCover {
        mime_type: "image/gif".to_string(),
        data: vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61], // GIF89a header
        duration_ms: Some(3000),
        loop_count: Some(0), // Infinite loop
    });

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert!(unpacked.animated_cover.is_some());
    assert_eq!(unpacked.animated_cover.unwrap().mime_type, "image/gif");
}

#[test]
fn test_cover_variants() {
    let mut meta = FloMetadata::new();

    // Add main cover
    meta.add_picture("image/jpeg", PictureType::CoverFront, vec![1, 2, 3]);

    // Add variants
    meta.cover_variants = vec![
        CoverVariant {
            variant_type: CoverVariantType::Explicit,
            mime_type: "image/jpeg".to_string(),
            data: vec![4, 5, 6],
            description: Some("Explicit version".to_string()),
        },
        CoverVariant {
            variant_type: CoverVariantType::Remix,
            mime_type: "image/png".to_string(),
            data: vec![7, 8, 9],
            description: Some("Remix artwork".to_string()),
        },
    ];

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.cover_variants.len(), 2);
    assert_eq!(
        unpacked.cover_variants[0].variant_type,
        CoverVariantType::Explicit
    );
}

#[test]
fn test_artist_signature() {
    let mut meta = FloMetadata::new();

    // Fake signature image
    meta.artist_signature = Some(Picture {
        mime_type: "image/png".to_string(),
        picture_type: PictureType::Other,
        description: Some("Artist signature".to_string()),
        data: vec![0x89, 0x50, 0x4E, 0x47],
    });

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert!(unpacked.artist_signature.is_some());
}

// ============================================================================
// Custom Fields Tests
// ============================================================================

#[test]
fn test_custom_fields() {
    let mut meta = FloMetadata::new();
    meta.set_custom("my_app_id", "12345");
    meta.set_custom("my_app_rating", "5");

    assert_eq!(meta.get_custom("my_app_id"), Some("12345"));
    assert_eq!(meta.get_custom("my_app_rating"), Some("5"));
    assert_eq!(meta.get_custom("nonexistent"), None);

    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    assert_eq!(unpacked.get_custom("my_app_id"), Some("12345"));
}

// ============================================================================
// Complex Roundtrip Test
// ============================================================================

#[test]
fn test_full_metadata_roundtrip() {
    let mut meta = FloMetadata::new();

    // Basic info
    meta.title = Some("Epic Track".to_string());
    meta.artist = Some("Super Producer".to_string());
    meta.album = Some("Best Album Ever".to_string());
    meta.year = Some(2026);
    meta.track_number = Some(5);
    meta.track_total = Some(12);
    meta.disc_number = Some(1);
    meta.disc_total = Some(2);
    meta.genre = Some("Electronic".to_string());
    meta.bpm = Some(128);
    meta.key = Some("Fm".to_string());

    // Pictures
    meta.add_picture("image/jpeg", PictureType::CoverFront, vec![1, 2, 3, 4]);

    // Comments and lyrics
    meta.add_comment("Production notes here", Some("eng"));
    // hehe
    meta.add_lyrics("La la la\nDa da da", Some("eng"));

    // Synced lyrics
    meta.synced_lyrics.push(SyncedLyrics {
        language: Some("eng".to_string()),
        content_type: SyncedLyricsContentType::Lyrics,
        description: None,
        lines: vec![
            SyncedLyricsLine {
                timestamp_ms: 0,
                text: "La la la".to_string(),
            },
            SyncedLyricsLine {
                timestamp_ms: 2000,
                text: "Da da da".to_string(),
            },
        ],
    });

    // flo-unique features
    meta.section_markers = vec![
        SectionMarker {
            timestamp_ms: 0,
            section_type: SectionType::Intro,
            label: None,
        },
        SectionMarker {
            timestamp_ms: 30000,
            section_type: SectionType::Drop,
            label: Some("Main Drop".to_string()),
        },
    ];

    meta.bpm_map = vec![BpmChange {
        timestamp_ms: 0,
        bpm: 128.0,
    }];

    // Roundtrip
    let packed = meta.to_msgpack().unwrap();
    let unpacked = FloMetadata::from_msgpack(&packed).unwrap();

    // Verify everything
    assert_eq!(unpacked.title, meta.title);
    assert_eq!(unpacked.artist, meta.artist);
    assert_eq!(unpacked.pictures.len(), 1);
    assert_eq!(unpacked.synced_lyrics.len(), 1);
    assert_eq!(unpacked.section_markers.len(), 2);
    assert_eq!(unpacked.bpm_map.len(), 1);
}

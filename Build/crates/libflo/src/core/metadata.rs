//! flo™ Metadata
//!
//! Supports most commonly used ID3v2.4 fields plus flo-unique extensions
//! Uses MessagePack serialization for efficiency and flexibility

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Picture Types (ID3v2.4 APIC)
// ============================================================================

/// Picture type (matches ID3v2.4 APIC picture types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PictureType {
    Other,
    FileIcon, // 32x32 PNG only
    OtherFileIcon,
    #[default]
    CoverFront,
    CoverBack,
    LeafletPage,
    Media, // e.g. label side of CD
    LeadArtist,
    Artist,
    Conductor,
    Band,
    Composer,
    Lyricist,
    RecordingLocation,
    DuringRecording,
    DuringPerformance,
    VideoScreenCapture,
    BrightColouredFish, // Yes, this is real in ID3v2.4 🐟
    Illustration,
    BandLogo,
    PublisherLogo,
}

/// Attached picture (album art, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Picture {
    /// MIME type (e.g., "image/jpeg", "image/png")
    pub mime_type: String,
    /// Picture type
    pub picture_type: PictureType,
    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Binary picture data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

// ============================================================================
// Text Structures
// ============================================================================

/// Comment with optional language and description (COMM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// ISO-639-2 language code (e.g., "eng", "jpn")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Short content description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The actual comment text
    pub text: String,
}

/// Unsynchronized lyrics (USLT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lyrics {
    /// ISO-639-2 language code
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Content description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Lyrics text
    pub text: String,
}

/// Synchronized lyrics content type (SYLT)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncedLyricsContentType {
    Other,
    #[default]
    Lyrics,
    TextTranscription,
    PartName, // e.g., "Adagio"
    Events,   // e.g., "Don Quijote enters the stage"
    Chord,    // e.g., "Bb F Fsus"
    Trivia,   // Pop-up information
    WebpageUrl,
    ImageUrl,
}

/// A single line of synchronized lyrics with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedLyricsLine {
    /// Timestamp in milliseconds from start
    pub timestamp_ms: u64,
    /// Text/syllable at this timestamp
    pub text: String,
}

/// Synchronized lyrics/text (SYLT): flo first-party support!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedLyrics {
    /// ISO-639-2 language code
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Content type
    #[serde(default)]
    pub content_type: SyncedLyricsContentType,
    /// Content description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Lines with timestamps
    pub lines: Vec<SyncedLyricsLine>,
}

/// User-defined text field (TXXX)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserText {
    /// Description/key
    pub description: String,
    /// Value
    pub value: String,
}

/// User-defined URL (WXXX)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUrl {
    /// Description
    pub description: String,
    /// URL
    pub url: String,
}

/// Popularimeter (POPM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Popularimeter {
    /// Email/identifier of the user
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Rating (1-255, 0 = unknown)
    pub rating: u8,
    /// Play counter
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_count: Option<u64>,
}

// ============================================================================
// flo-Unique Structures
// ============================================================================

/// Pre-computed waveform data for instant visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformData {
    /// Number of peak values per second of audio
    pub peaks_per_second: u32,
    /// Peak values (0.0 to 1.0): for stereo, interleaved L/R or combined
    pub peaks: Vec<f32>,
    /// Number of channels in peaks (1 = mono/combined, 2 = stereo)
    #[serde(default = "default_one")]
    pub channels: u8,
}

fn default_one() -> u8 {
    1
}

/// Section type for track structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionType {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    PostChorus,
    Bridge,
    Breakdown,
    Drop,
    Buildup,
    Solo,
    Instrumental,
    Outro,
    Silence,
    Other,
}

/// Section marker with timestamp and label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionMarker {
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Section type
    pub section_type: SectionType,
    /// Optional custom label (e.g., "Verse 2", "Guitar Solo")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// BPM change point for tempo mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BpmChange {
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// BPM at this point (supports fractional BPM)
    pub bpm: f32,
}

/// Key change point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyChange {
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Musical key (e.g., "Am", "F#m", "Bb")
    pub key: String,
}

/// Loudness measurement point for dynamic visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessPoint {
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Loudness in LUFS
    pub lufs: f32,
}

/// Creator/producer note with optional timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorNote {
    /// Optional timestamp (None = applies to whole track)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<u64>,
    /// Note text
    pub text: String,
}

/// Collaboration credit with role and optional timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationCredit {
    /// Role (e.g., "Lead Vocals", "Bass Guitar", "Mixing")
    pub role: String,
    /// Person's name
    pub name: String,
    /// Optional timestamp for when they appear (e.g., guitar solo at 2:00)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<u64>,
}

/// Entry in remix/sample chain for tracking lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemixChainEntry {
    /// Original track title
    pub title: String,
    /// Original artist
    pub artist: String,
    /// Year of original (if known)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,
    /// ISRC of original (if known)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
    /// Relationship type: "original", "remix", "sample", "cover", "mashup"
    pub relationship: String,
}

/// Animated cover art (GIF, animated WebP, or short video)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatedCover {
    /// MIME type (image/gif, image/webp, video/mp4)
    pub mime_type: String,
    /// Binary data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// Duration in milliseconds (if applicable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
    /// Loop count (0 = infinite, None = play once)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_count: Option<u32>,
}

/// Cover variant type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverVariantType {
    Standard,
    Explicit,
    Clean,
    Remix,
    Deluxe,
    Limited,
    Vinyl,
    Cassette,
    Digital,
    Other,
}

/// Alternative cover art variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverVariant {
    /// Variant type
    pub variant_type: CoverVariantType,
    /// MIME type
    pub mime_type: String,
    /// Binary data
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================================
// Main Metadata Structure
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FloMetadata {
    // ==================== IDENTIFICATION ====================
    /// Title/songname (TIT2)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Subtitle/description refinement (TIT3)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,

    /// Content group description (TIT1)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_group: Option<String>,

    /// Album/movie/show title (TALB)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,

    /// Original album (TOAL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_album: Option<String>,

    /// Set subtitle (TSST)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_subtitle: Option<String>,

    /// Track number (TRCK)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_number: Option<u32>,

    /// Total tracks
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_total: Option<u32>,

    /// Disc number (TPOS)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc_number: Option<u32>,

    /// Total discs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc_total: Option<u32>,

    /// ISRC code (TSRC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,

    // ==================== INVOLVED PERSONS ====================
    /// Lead artist/performer (TPE1)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,

    /// Album artist/band (TPE2)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_artist: Option<String>,

    /// Conductor (TPE3)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conductor: Option<String>,

    /// Remixer/modifier (TPE4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remixer: Option<String>,

    /// Original artist (TOPE)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_artist: Option<String>,

    /// Composer (TCOM)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,

    /// Lyricist/text writer (TEXT)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,

    /// Original lyricist (TOLY)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_lyricist: Option<String>,

    /// Encoded by (TENC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoded_by: Option<String>,

    /// Involved people list (TIPL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub involved_people: Option<Vec<(String, String)>>,

    /// Musician credits (TMCL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub musician_credits: Option<Vec<(String, String)>>,

    // ==================== PROPERTIES ====================
    /// Genre (TCON)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    /// Mood (TMOO)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood: Option<String>,

    /// BPM (TBPM)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bpm: Option<u32>,

    /// Initial musical key (TKEY)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Language (TLAN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Length in milliseconds (TLEN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_ms: Option<u64>,

    // ==================== DATES/TIMES ====================
    /// Year
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,

    /// Recording time (TDRC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_time: Option<String>,

    /// Release time (TDRL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_time: Option<String>,

    /// Original release time (TDOR)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_release_time: Option<String>,

    /// Encoding time (TDEN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding_time: Option<String>,

    /// Tagging time (TDTG)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tagging_time: Option<String>,

    // ==================== RIGHTS/LICENSE ====================
    /// Copyright message (TCOP)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,

    /// Production copyright (TPRO)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced_notice: Option<String>,

    /// Publisher (TPUB)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    /// File owner/licensee (TOWN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_owner: Option<String>,

    /// Internet radio station name (TRSN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio_station: Option<String>,

    /// Internet radio station owner (TRSO)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio_station_owner: Option<String>,

    // ==================== SORT ORDER ====================
    /// Album sort order (TSOA)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_sort: Option<String>,

    /// Performer sort order (TSOP)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist_sort: Option<String>,

    /// Title sort order (TSOT)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_sort: Option<String>,

    // ==================== OTHER TEXT ====================
    /// Original filename (TOFN)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,

    /// Playlist delay in ms (TDLY)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playlist_delay: Option<u32>,

    /// Encoder software/settings (TSSE)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoder_settings: Option<String>,

    // ==================== URLS ====================
    /// Commercial info URL (WCOM)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_commercial: Option<String>,

    /// Copyright/legal URL (WCOP)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_copyright: Option<String>,

    /// Official audio file URL (WOAF)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_audio_file: Option<String>,

    /// Official artist URL (WOAR)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_artist: Option<String>,

    /// Official audio source URL (WOAS)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_audio_source: Option<String>,

    /// Official radio station URL (WORS)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_radio_station: Option<String>,

    /// Payment URL (WPAY)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_payment: Option<String>,

    /// Publisher URL (WPUB)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_publisher: Option<String>,

    /// User-defined URLs (WXXX)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_urls: Vec<UserUrl>,

    // ==================== COMPLEX FRAMES ====================
    /// Comments (COMM)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,

    /// Unsynchronized lyrics (USLT)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lyrics: Vec<Lyrics>,

    /// Synchronized lyrics (SYLT)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub synced_lyrics: Vec<SyncedLyrics>,

    /// Attached pictures (APIC)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pictures: Vec<Picture>,

    /// User-defined text (TXXX)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_text: Vec<UserText>,

    /// Play counter (PCNT)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play_count: Option<u64>,

    /// Popularimeter/rating (POPM)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popularimeter: Option<Popularimeter>,

    // ==================== VISUALIZATION (flo-unique) ====================
    /// Pre-computed waveform peaks for instant visualization
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform_data: Option<WaveformData>,

    /// Spectral analysis data / audio fingerprint for visual EQ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(with = "serde_bytes_option")]
    pub spectrum_fingerprint: Option<Vec<u8>>,

    // ==================== TIMING & ANALYSIS (flo-unique) ====================
    /// BPM changes throughout the track
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bpm_map: Vec<BpmChange>,

    /// Musical key changes with timestamps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_changes: Vec<KeyChange>,

    /// Frame-by-frame loudness profile (ReplayGain dynamic)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loudness_profile: Vec<LoudnessPoint>,

    /// Integrated loudness (LUFS): EBU R128
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrated_loudness_lufs: Option<f32>,

    /// Loudness range (LU)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness_range_lu: Option<f32>,

    /// True peak (dBTP)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_peak_dbtp: Option<f32>,

    /// Section markers (intro/verse/chorus/etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub section_markers: Vec<SectionMarker>,

    // ==================== CREATOR INFO (flo™-unique) ====================
    /// Producer commentary with timestamps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creator_notes: Vec<CreatorNote>,

    /// Detailed collaboration credits
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collaboration_credits: Vec<CollaborationCredit>,

    /// Remix/sample lineage chain
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remix_chain: Vec<RemixChainEntry>,

    // ==================== COVERS (flo™-unique) ====================
    /// Animated cover art (GIF/WebP/short video)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animated_cover: Option<AnimatedCover>,

    /// Alternative cover variants (explicit, remix, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cover_variants: Vec<CoverVariant>,

    /// Artist signature image
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist_signature: Option<Picture>,

    // ==================== flo™-SPECIFIC ====================
    /// flo encoder version used
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flo_encoder_version: Option<String>,

    /// Source format (e.g., "MP3", "FLAC", "WAV")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_format: Option<String>,

    /// Custom key-value pairs for extensions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

// Helper for Option<Vec<u8>> serialization
mod serde_bytes_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(data: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(bytes) => serde_bytes::serialize(bytes, serializer),
            None => Option::<&[u8]>::None.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<serde_bytes::ByteBuf> = Option::deserialize(deserializer)?;
        Ok(opt.map(|b| b.into_vec()))
    }
}

impl FloMetadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Create metadata with basic fields
    pub fn with_basic(
        title: Option<String>,
        artist: Option<String>,
        album: Option<String>,
    ) -> Self {
        Self {
            title,
            artist,
            album,
            ..Default::default()
        }
    }

    /// Serialize to MessagePack bytes
    pub fn to_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Deserialize from MessagePack bytes
    pub fn from_msgpack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Check if metadata is empty (no significant fields set)
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.pictures.is_empty()
            && self.comments.is_empty()
            && self.lyrics.is_empty()
            && self.synced_lyrics.is_empty()
    }

    // ==================== PICTURE HELPERS ====================

    /// Add a picture
    pub fn add_picture(&mut self, mime_type: &str, picture_type: PictureType, data: Vec<u8>) {
        self.pictures.push(Picture {
            mime_type: mime_type.to_string(),
            picture_type,
            description: None,
            data,
        });
    }

    /// Get the front cover picture if present
    pub fn front_cover(&self) -> Option<&Picture> {
        self.pictures
            .iter()
            .find(|p| p.picture_type == PictureType::CoverFront)
    }

    /// Get the first picture of any type
    pub fn any_picture(&self) -> Option<&Picture> {
        self.pictures.first()
    }

    // ==================== TEXT HELPERS ====================

    /// Add a comment
    pub fn add_comment(&mut self, text: &str, language: Option<&str>) {
        self.comments.push(Comment {
            language: language.map(|s| s.to_string()),
            description: None,
            text: text.to_string(),
        });
    }

    /// Add unsynchronized lyrics
    pub fn add_lyrics(&mut self, text: &str, language: Option<&str>) {
        self.lyrics.push(Lyrics {
            language: language.map(|s| s.to_string()),
            description: None,
            text: text.to_string(),
        });
    }

    /// Add synchronized lyrics line
    pub fn add_synced_lyrics_line(
        &mut self,
        timestamp_ms: u64,
        text: &str,
        language: Option<&str>,
    ) {
        let lang = language.map(|s| s.to_string());
        if let Some(synced) = self.synced_lyrics.iter_mut().find(|s| s.language == lang) {
            synced.lines.push(SyncedLyricsLine {
                timestamp_ms,
                text: text.to_string(),
            });
        } else {
            self.synced_lyrics.push(SyncedLyrics {
                language: lang,
                content_type: SyncedLyricsContentType::Lyrics,
                description: None,
                lines: vec![SyncedLyricsLine {
                    timestamp_ms,
                    text: text.to_string(),
                }],
            });
        }
    }

    // ==================== CUSTOM FIELD HELPERS ====================

    /// Set a custom field
    pub fn set_custom(&mut self, key: &str, value: &str) {
        self.custom.insert(key.to_string(), value.to_string());
    }

    /// Get a custom field
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom.get(key).map(|s| s.as_str())
    }

    // ==================== HELPERS (flo™-unique) ====================

    /// Add a section marker
    pub fn add_section(
        &mut self,
        timestamp_ms: u64,
        section_type: SectionType,
        label: Option<&str>,
    ) {
        self.section_markers.push(SectionMarker {
            timestamp_ms,
            section_type,
            label: label.map(|s| s.to_string()),
        });
    }

    /// Add a BPM change point
    pub fn add_bpm_change(&mut self, timestamp_ms: u64, bpm: f32) {
        self.bpm_map.push(BpmChange { timestamp_ms, bpm });
    }

    /// Add a key change point
    pub fn add_key_change(&mut self, timestamp_ms: u64, key: &str) {
        self.key_changes.push(KeyChange {
            timestamp_ms,
            key: key.to_string(),
        });
    }

    /// Add a creator note
    pub fn add_creator_note(&mut self, text: &str, timestamp_ms: Option<u64>) {
        self.creator_notes.push(CreatorNote {
            timestamp_ms,
            text: text.to_string(),
        });
    }

    /// Add collaboration credit
    pub fn add_collaboration(&mut self, role: &str, name: &str, timestamp_ms: Option<u64>) {
        self.collaboration_credits.push(CollaborationCredit {
            role: role.to_string(),
            name: name.to_string(),
            timestamp_ms,
        });
    }
}

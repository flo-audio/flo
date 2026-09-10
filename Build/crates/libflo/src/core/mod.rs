pub mod analysis;
pub mod audio_constants;
pub mod crc32;
pub mod ebu_r128;
pub mod metadata;
pub mod rice;
pub mod types;

pub use analysis::*;
pub use audio_constants::*;
pub use crc32::compute as compute_crc32;

pub use rice::{
    decode as rice_decode, decode_i32 as rice_decode_i32, encode as rice_encode,
    encode_i32 as rice_encode_i32, estimate_rice_parameter, estimate_rice_parameter_i32, BitReader,
    BitWriter,
};

pub use types::*;

pub use metadata::{
    AnimatedCover, BpmChange, CollaborationCredit, Comment, CoverVariant, CoverVariantType,
    CreatorNote, FloMetadata, KeyChange, LoudnessPoint, Lyrics, Picture, PictureType,
    Popularimeter, RemixChainEntry, SectionMarker, SectionType, SyncedLyrics,
    SyncedLyricsContentType, SyncedLyricsLine, UserText, UserUrl, WaveformData,
};

pub use analysis::{
    extract_dominant_frequencies, extract_spectral_fingerprint, extract_waveform_peaks,
    extract_waveform_rms, spectral_similarity, SpectralFingerprint,
};

pub use ebu_r128::{compute_ebu_r128_loudness, LoudnessMetrics};

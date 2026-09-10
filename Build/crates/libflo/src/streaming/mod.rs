//! streaming encode/decode for flo audio
//!
//! incremental encoding and decoding for network streaming or memory constrained stuff
mod decoder;
mod encoder;
mod types;

pub use decoder::StreamingDecoder;
pub use encoder::{EncodedFrame, StreamingEncoder};
pub use types::{DecoderState, StreamingAudioInfo};

#[cfg(test)]
mod tests;

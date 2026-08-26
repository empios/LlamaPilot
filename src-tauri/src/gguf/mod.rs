//! Lightweight GGUF inspection used by the model catalog.
//!
//! The reader deliberately stops after the key/value metadata block. It never parses tensor
//! descriptors and never maps or reads tensor payloads.

mod reader;
mod split;

pub use reader::{read_metadata, GgufError, GgufMetadata};
pub use split::{parse_split_filename, SplitName};

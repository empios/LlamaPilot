use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const MIN_SUPPORTED_VERSION: u32 = 2;
const MAX_SUPPORTED_VERSION: u32 = 3;
const MAX_METADATA_ENTRIES: u64 = 1_000_000;
const MAX_KEY_BYTES: u64 = 64 * 1024;
const MAX_CAPTURED_STRING_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ARRAY_ENTRIES: u64 = 100_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GgufMetadata {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_count: u64,
    /// Byte offset immediately after the metadata block. Tensor descriptors start here.
    pub metadata_bytes: u64,
    pub general_type: Option<String>,
    pub architecture: Option<String>,
    pub name: Option<String>,
    pub basename: Option<String>,
    pub size_label: Option<String>,
    pub file_type: Option<u32>,
    pub quantization_version: Option<u32>,
    pub context_length: Option<u64>,
    pub embedding_length: Option<u64>,
    pub block_count: Option<u64>,
    pub tokenizer_model: Option<String>,
    pub split_index: Option<u32>,
    pub split_count: Option<u32>,
    pub split_tensor_count: Option<u64>,
    pub projector_type: Option<String>,
    pub has_vision_encoder: Option<bool>,
    pub has_audio_encoder: Option<bool>,
}

#[derive(Debug)]
pub enum GgufError {
    Io(std::io::Error),
    Invalid(String),
}

impl fmt::Display for GgufError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GgufError {}

impl From<std::io::Error> for GgufError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum ValueType {
    Uint8 = 0,
    Int8 = 1,
    Uint16 = 2,
    Int16 = 3,
    Uint32 = 4,
    Int32 = 5,
    Float32 = 6,
    Bool = 7,
    String = 8,
    Array = 9,
    Uint64 = 10,
    Int64 = 11,
    Float64 = 12,
}

impl TryFrom<u32> for ValueType {
    type Error = GgufError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Uint8),
            1 => Ok(Self::Int8),
            2 => Ok(Self::Uint16),
            3 => Ok(Self::Int16),
            4 => Ok(Self::Uint32),
            5 => Ok(Self::Int32),
            6 => Ok(Self::Float32),
            7 => Ok(Self::Bool),
            8 => Ok(Self::String),
            9 => Ok(Self::Array),
            10 => Ok(Self::Uint64),
            11 => Ok(Self::Int64),
            12 => Ok(Self::Float64),
            other => Err(GgufError::Invalid(format!(
                "Unknown GGUF metadata value type {other}."
            ))),
        }
    }
}

#[derive(Debug)]
enum Scalar {
    Unsigned(u64),
    Signed(i64),
    Float,
    Boolean(bool),
}

impl Scalar {
    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Unsigned(value) => Some(*value),
            Self::Signed(value) => u64::try_from(*value).ok(),
            Self::Float | Self::Boolean(_) => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }
}

struct Reader {
    inner: BufReader<File>,
    length: u64,
    position: u64,
}

impl Reader {
    fn open(path: &Path) -> Result<Self, GgufError> {
        let file = File::open(path)?;
        let length = file.metadata()?.len();
        Ok(Self {
            inner: BufReader::new(file),
            length,
            position: 0,
        })
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], GgufError> {
        self.ensure_available(N as u64)?;
        let mut bytes = [0_u8; N];
        self.inner.read_exact(&mut bytes)?;
        self.position += N as u64;
        Ok(bytes)
    }

    fn read_bytes(&mut self, length: u64) -> Result<Vec<u8>, GgufError> {
        self.ensure_available(length)?;
        let allocation = usize::try_from(length)
            .map_err(|_| GgufError::Invalid("GGUF string is too large.".to_string()))?;
        let mut bytes = vec![0_u8; allocation];
        self.inner.read_exact(&mut bytes)?;
        self.position += length;
        Ok(bytes)
    }

    fn skip(&mut self, length: u64) -> Result<(), GgufError> {
        self.ensure_available(length)?;
        let offset = i64::try_from(length)
            .map_err(|_| GgufError::Invalid("GGUF value is too large to skip.".to_string()))?;
        self.inner.seek(SeekFrom::Current(offset))?;
        self.position += length;
        Ok(())
    }

    fn ensure_available(&self, length: u64) -> Result<(), GgufError> {
        if self
            .position
            .checked_add(length)
            .is_none_or(|end| end > self.length)
        {
            return Err(GgufError::Invalid(
                "GGUF metadata ends unexpectedly.".to_string(),
            ));
        }
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, GgufError> {
        Ok(self.read_exact::<1>()?[0])
    }

    fn i8(&mut self) -> Result<i8, GgufError> {
        Ok(i8::from_le_bytes(self.read_exact()?))
    }

    fn u16(&mut self) -> Result<u16, GgufError> {
        Ok(u16::from_le_bytes(self.read_exact()?))
    }

    fn i16(&mut self) -> Result<i16, GgufError> {
        Ok(i16::from_le_bytes(self.read_exact()?))
    }

    fn u32(&mut self) -> Result<u32, GgufError> {
        Ok(u32::from_le_bytes(self.read_exact()?))
    }

    fn i32(&mut self) -> Result<i32, GgufError> {
        Ok(i32::from_le_bytes(self.read_exact()?))
    }

    fn u64(&mut self) -> Result<u64, GgufError> {
        Ok(u64::from_le_bytes(self.read_exact()?))
    }

    fn i64(&mut self) -> Result<i64, GgufError> {
        Ok(i64::from_le_bytes(self.read_exact()?))
    }

    fn string(&mut self, maximum: u64) -> Result<String, GgufError> {
        let length = self.u64()?;
        if length > maximum {
            return Err(GgufError::Invalid(format!(
                "GGUF string length {length} exceeds the safe inspection limit."
            )));
        }
        let bytes = self.read_bytes(length)?;
        String::from_utf8(bytes)
            .map_err(|_| GgufError::Invalid("GGUF metadata contains invalid UTF-8.".to_string()))
    }

    fn optional_string(&mut self, capture: bool) -> Result<Option<String>, GgufError> {
        let length = self.u64()?;
        if capture && length <= MAX_CAPTURED_STRING_BYTES {
            let bytes = self.read_bytes(length)?;
            let value = String::from_utf8(bytes).map_err(|_| {
                GgufError::Invalid("GGUF metadata contains invalid UTF-8.".to_string())
            })?;
            Ok(Some(value))
        } else {
            self.skip(length)?;
            Ok(None)
        }
    }
}

pub fn read_metadata(path: &Path) -> Result<GgufMetadata, GgufError> {
    let mut reader = Reader::open(path)?;
    if reader.read_exact::<4>()? != *GGUF_MAGIC {
        return Err(GgufError::Invalid(
            "The file does not start with the GGUF magic value.".to_string(),
        ));
    }

    let version = reader.u32()?;
    if !(MIN_SUPPORTED_VERSION..=MAX_SUPPORTED_VERSION).contains(&version) {
        return Err(GgufError::Invalid(format!(
            "GGUF version {version} is not supported (expected version 2 or 3)."
        )));
    }

    let tensor_count = non_negative_count(reader.i64()?, "tensor")?;
    let metadata_count = non_negative_count(reader.i64()?, "metadata")?;
    if metadata_count > MAX_METADATA_ENTRIES {
        return Err(GgufError::Invalid(format!(
            "GGUF contains an unreasonable number of metadata entries ({metadata_count})."
        )));
    }

    let mut metadata = GgufMetadata {
        version,
        tensor_count,
        metadata_count,
        ..GgufMetadata::default()
    };
    let mut numeric_fields = BTreeMap::<String, u64>::new();

    for _ in 0..metadata_count {
        let key = reader.string(MAX_KEY_BYTES)?;
        let value_type = ValueType::try_from(reader.u32()?)?;
        match value_type {
            ValueType::String => {
                let capture = is_interesting_string(&key);
                if let Some(value) = reader.optional_string(capture)? {
                    assign_string(&mut metadata, &key, value);
                }
            }
            ValueType::Array => skip_array(&mut reader)?,
            scalar_type => {
                let scalar = read_scalar(&mut reader, scalar_type)?;
                if let Some(value) = scalar.as_u64() {
                    numeric_fields.insert(key.clone(), value);
                }
                assign_scalar(&mut metadata, &key, &scalar);
            }
        }
    }

    let architecture = metadata.architecture.as_deref();
    metadata.context_length = architecture_field(&numeric_fields, architecture, "context_length");
    metadata.embedding_length =
        architecture_field(&numeric_fields, architecture, "embedding_length");
    metadata.block_count = architecture_field(&numeric_fields, architecture, "block_count");
    metadata.metadata_bytes = reader.position;
    Ok(metadata)
}

fn non_negative_count(value: i64, label: &str) -> Result<u64, GgufError> {
    u64::try_from(value)
        .map_err(|_| GgufError::Invalid(format!("GGUF declares a negative {label} count.")))
}

fn is_interesting_string(key: &str) -> bool {
    matches!(
        key,
        "general.type"
            | "general.architecture"
            | "general.name"
            | "general.basename"
            | "general.size_label"
            | "tokenizer.ggml.model"
            | "clip.projector_type"
    )
}

fn assign_string(metadata: &mut GgufMetadata, key: &str, value: String) {
    match key {
        "general.type" => metadata.general_type = Some(value),
        "general.architecture" => metadata.architecture = Some(value),
        "general.name" => metadata.name = Some(value),
        "general.basename" => metadata.basename = Some(value),
        "general.size_label" => metadata.size_label = Some(value),
        "tokenizer.ggml.model" => metadata.tokenizer_model = Some(value),
        "clip.projector_type" => metadata.projector_type = Some(value),
        _ => {}
    }
}

fn assign_scalar(metadata: &mut GgufMetadata, key: &str, scalar: &Scalar) {
    match key {
        "general.file_type" => metadata.file_type = scalar.as_u64().and_then(|v| v.try_into().ok()),
        "general.quantization_version" => {
            metadata.quantization_version = scalar.as_u64().and_then(|v| v.try_into().ok());
        }
        "split.no" => metadata.split_index = scalar.as_u64().and_then(|v| v.try_into().ok()),
        "split.count" => metadata.split_count = scalar.as_u64().and_then(|v| v.try_into().ok()),
        "split.tensors.count" => metadata.split_tensor_count = scalar.as_u64(),
        "clip.has_vision_encoder" => metadata.has_vision_encoder = scalar.as_bool(),
        "clip.has_audio_encoder" => metadata.has_audio_encoder = scalar.as_bool(),
        _ => {}
    }
}

fn architecture_field(
    fields: &BTreeMap<String, u64>,
    architecture: Option<&str>,
    suffix: &str,
) -> Option<u64> {
    architecture
        .and_then(|architecture| fields.get(&format!("{architecture}.{suffix}")).copied())
        .or_else(|| {
            fields
                .iter()
                .find_map(|(key, value)| key.ends_with(&format!(".{suffix}")).then_some(*value))
        })
}

fn read_scalar(reader: &mut Reader, value_type: ValueType) -> Result<Scalar, GgufError> {
    match value_type {
        ValueType::Uint8 => Ok(Scalar::Unsigned(u64::from(reader.u8()?))),
        ValueType::Int8 => Ok(Scalar::Signed(i64::from(reader.i8()?))),
        ValueType::Uint16 => Ok(Scalar::Unsigned(u64::from(reader.u16()?))),
        ValueType::Int16 => Ok(Scalar::Signed(i64::from(reader.i16()?))),
        ValueType::Uint32 => Ok(Scalar::Unsigned(u64::from(reader.u32()?))),
        ValueType::Int32 => Ok(Scalar::Signed(i64::from(reader.i32()?))),
        ValueType::Float32 => {
            reader.read_exact::<4>()?;
            Ok(Scalar::Float)
        }
        ValueType::Bool => Ok(Scalar::Boolean(reader.u8()? != 0)),
        ValueType::Uint64 => Ok(Scalar::Unsigned(reader.u64()?)),
        ValueType::Int64 => Ok(Scalar::Signed(reader.i64()?)),
        ValueType::Float64 => {
            reader.read_exact::<8>()?;
            Ok(Scalar::Float)
        }
        ValueType::String | ValueType::Array => Err(GgufError::Invalid(
            "Internal GGUF scalar parser mismatch.".to_string(),
        )),
    }
}

fn skip_array(reader: &mut Reader) -> Result<(), GgufError> {
    let element_type = ValueType::try_from(reader.u32()?)?;
    if element_type == ValueType::Array {
        return Err(GgufError::Invalid(
            "Nested GGUF metadata arrays are not supported by the format.".to_string(),
        ));
    }
    let count = reader.u64()?;
    if count > MAX_ARRAY_ENTRIES {
        return Err(GgufError::Invalid(format!(
            "GGUF metadata array contains an unreasonable number of values ({count})."
        )));
    }

    if element_type == ValueType::String {
        for _ in 0..count {
            let length = reader.u64()?;
            reader.skip(length)?;
        }
        return Ok(());
    }

    let width = scalar_width(element_type)?;
    let bytes = count
        .checked_mul(width)
        .ok_or_else(|| GgufError::Invalid("GGUF metadata array size overflows.".to_string()))?;
    reader.skip(bytes)
}

fn scalar_width(value_type: ValueType) -> Result<u64, GgufError> {
    match value_type {
        ValueType::Uint8 | ValueType::Int8 | ValueType::Bool => Ok(1),
        ValueType::Uint16 | ValueType::Int16 => Ok(2),
        ValueType::Uint32 | ValueType::Int32 | ValueType::Float32 => Ok(4),
        ValueType::Uint64 | ValueType::Int64 | ValueType::Float64 => Ok(8),
        ValueType::String | ValueType::Array => Err(GgufError::Invalid(
            "GGUF array has an invalid element type.".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn push_string(buffer: &mut Vec<u8>, value: &str) {
        buffer.extend_from_slice(&(value.len() as u64).to_le_bytes());
        buffer.extend_from_slice(value.as_bytes());
    }

    fn push_string_entry(buffer: &mut Vec<u8>, key: &str, value: &str) {
        push_string(buffer, key);
        buffer.extend_from_slice(&(ValueType::String as u32).to_le_bytes());
        push_string(buffer, value);
    }

    fn push_u32_entry(buffer: &mut Vec<u8>, key: &str, value: u32) {
        push_string(buffer, key);
        buffer.extend_from_slice(&(ValueType::Uint32 as u32).to_le_bytes());
        buffer.extend_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn reads_selected_metadata_and_stops_before_tensor_data() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(GGUF_MAGIC);
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&2_i64.to_le_bytes());
        bytes.extend_from_slice(&5_i64.to_le_bytes());
        push_string_entry(&mut bytes, "general.architecture", "llama");
        push_string_entry(&mut bytes, "general.name", "Tiny Model");
        push_u32_entry(&mut bytes, "general.file_type", 15);
        push_u32_entry(&mut bytes, "llama.context_length", 8192);
        push_u32_entry(&mut bytes, "split.count", 1);
        let metadata_end = bytes.len() as u64;
        bytes.extend_from_slice(b"this is deliberately not a valid tensor descriptor");

        let mut file = NamedTempFile::new().expect("temporary file");
        file.write_all(&bytes).expect("fixture written");

        let parsed = read_metadata(file.path()).expect("valid metadata");
        assert_eq!(parsed.name.as_deref(), Some("Tiny Model"));
        assert_eq!(parsed.architecture.as_deref(), Some("llama"));
        assert_eq!(parsed.context_length, Some(8192));
        assert_eq!(parsed.file_type, Some(15));
        assert_eq!(parsed.metadata_bytes, metadata_end);
        assert!(parsed.metadata_bytes < bytes.len() as u64);
    }

    #[test]
    fn skips_large_token_arrays_without_collecting_them() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(GGUF_MAGIC);
        bytes.extend_from_slice(&3_u32.to_le_bytes());
        bytes.extend_from_slice(&0_i64.to_le_bytes());
        bytes.extend_from_slice(&2_i64.to_le_bytes());
        push_string(&mut bytes, "tokenizer.ggml.tokens");
        bytes.extend_from_slice(&(ValueType::Array as u32).to_le_bytes());
        bytes.extend_from_slice(&(ValueType::String as u32).to_le_bytes());
        bytes.extend_from_slice(&3_u64.to_le_bytes());
        push_string(&mut bytes, "one");
        push_string(&mut bytes, "two");
        push_string(&mut bytes, "three");
        push_string_entry(&mut bytes, "general.name", "After Tokens");

        let mut file = NamedTempFile::new().expect("temporary file");
        file.write_all(&bytes).expect("fixture written");

        let parsed = read_metadata(file.path()).expect("valid metadata");
        assert_eq!(parsed.name.as_deref(), Some("After Tokens"));
        assert_eq!(parsed.metadata_bytes, bytes.len() as u64);
    }

    #[test]
    fn rejects_non_gguf_files_and_truncated_metadata() {
        let mut wrong = NamedTempFile::new().expect("temporary file");
        wrong.write_all(b"nope").expect("fixture written");
        assert!(read_metadata(wrong.path()).is_err());

        let mut truncated = NamedTempFile::new().expect("temporary file");
        truncated
            .write_all(b"GGUF\x03\x00")
            .expect("fixture written");
        assert!(read_metadata(truncated.path()).is_err());
    }
}

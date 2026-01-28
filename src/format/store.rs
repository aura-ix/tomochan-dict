use crate::schema::BINCODE_CONFIG;
use std::io::Write;
use zeekstd::{Decoder, EncodeOptions};

const ZSTD_COMPRESSION_LEVEL: i32 = 19;

pub struct UnifiedStoreBuilder {
    buffer: Vec<u8>,
    current_offset: u64,
}

impl UnifiedStoreBuilder {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            buffer: Vec::new(),
            current_offset: 0,
        })
    }

    pub fn insert<T: bincode::Encode>(&mut self, item: &T) -> Result<u64, String> {
        let serialized = bincode::encode_to_vec(item, BINCODE_CONFIG)
            .map_err(|e| format!("bincode serialization failed: {}", e))?;

        let offset = self.current_offset;
        let len = serialized.len() as u64;

        self.buffer.extend_from_slice(&serialized);
        self.current_offset += len;

        Ok(offset)
    }

    pub fn finalize(self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();
        let mut encoder = EncodeOptions::new()
            .compression_level(ZSTD_COMPRESSION_LEVEL)
            .into_encoder(&mut buffer)
            .map_err(|e| format!("Failed to create Encoder: {}", e))?;
        
        encoder.write_all(&self.buffer)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        
        encoder.finish()
            .map_err(|e| format!("Failed to finalize compressed data: {}", e))?;

        Ok(buffer)
    }
}

pub struct UnifiedStore {
    decoder: Decoder<'static, std::io::Cursor<Vec<u8>>>,
}

impl UnifiedStore {
    pub fn new(data: Vec<u8>) -> Result<Self, String> {
        let cursor = std::io::Cursor::new(data);
        let decoder = Decoder::new(cursor)
            .map_err(|e| format!("Failed to create Decoder: {}", e))?;

        Ok(Self { decoder })
    }

    pub fn get<T: bincode::Decode<()>>(&mut self, offset: u64) -> Result<T, String> {
        self.decoder.set_offset(offset)
            .map_err(|e| format!("Failed to seek to offset: {}", e))?;
        
        let (item, _): (T, usize) = bincode::decode_from_std_read(&mut self.decoder, BINCODE_CONFIG)
            .map_err(|e| format!("bincode deserialization failed: {}", e))?;
        
        Ok(item)
    }
}
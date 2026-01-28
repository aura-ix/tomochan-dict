use super::types::{TermId, KanjiId, TagId, TermMetaId, KanjiMetaId};
use crate::schema::{Term, Kanji, Tag, TermMeta, KanjiMeta, BINCODE_CONFIG};
use std::io::Write;
use zeekstd::{Encoder, Decoder, EncodeOptions};

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
            .map_err(|e| format!("Failed to finalize compressed data: {}", e));

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
    
    pub fn get_term(&mut self, id: TermId) -> Result<Term, String> {
        self.get(id.0)
    }
    
    pub fn get_kanji(&mut self, id: KanjiId) -> Result<Kanji, String> {
        self.get(id.0)
    }
    
    pub fn get_tag(&mut self, id: TagId) -> Result<Tag, String> {
        self.get(id.0)
    }
    
    pub fn get_term_meta(&mut self, id: TermMetaId) -> Result<TermMeta, String> {
        self.get(id.0)
    }
    
    pub fn get_kanji_meta(&mut self, id: KanjiMetaId) -> Result<KanjiMeta, String> {
        self.get(id.0)
    }
    
    pub fn get_extra_file(&mut self, offset: u64) -> Result<Vec<u8>, String> {
        self.get(offset)
    }
}
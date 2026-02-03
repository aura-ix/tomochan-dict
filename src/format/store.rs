use crate::schema::BINCODE_CONFIG;
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use zeekstd::{Decoder, EncodeOptions};

const ZSTD_COMPRESSION_LEVEL: i32 = 19;

// i cannot believe i need to do this :(
struct SharedBuffer(Rc<RefCell<Vec<u8>>>);

impl Write for SharedBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct UnifiedStoreBuilder {
    encoder: zeekstd::Encoder<'static, SharedBuffer>,
    buffer: Rc<RefCell<Vec<u8>>>,
    current_offset: u64,
}

impl UnifiedStoreBuilder {
    pub fn new() -> Result<Self, String> {
        let buffer = Rc::new(RefCell::new(Vec::new()));
        let writer = SharedBuffer(Rc::clone(&buffer));

        let encoder = EncodeOptions::new()
            .compression_level(ZSTD_COMPRESSION_LEVEL)
            .into_encoder(writer)
            .map_err(|e| format!("Failed to create Encoder: {}", e))?;

        Ok(Self {
            encoder,
            buffer,
            current_offset: 0,
        })
    }

    pub fn insert<T: bincode::Encode>(&mut self, item: &T) -> Result<u64, String> {
        let serialized = bincode::encode_to_vec(item, BINCODE_CONFIG)
            .map_err(|e| format!("bincode serialization failed: {}", e))?;

        let offset = self.current_offset;
        let len = serialized.len() as u64;

        self.encoder
            .write_all(&serialized)
            .map_err(|e| format!("Failed to write data: {}", e))?;

        self.current_offset += len;
        Ok(offset)
    }

    pub fn finalize(self) -> Result<Vec<u8>, String> {
        self.encoder
            .finish()
            .map_err(|e| format!("Failed to finalize compressed data: {}", e))?;

        Rc::try_unwrap(self.buffer)
            .map_err(|_| "Failed to extract buffer".to_string())
            .map(|refcell| refcell.into_inner())
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
        self.decoder
            .set_offset(offset)
            .map_err(|e| format!("Failed to seek to offset: {}", e))?;

        let (item, _): (T, usize) =
            bincode::decode_from_std_read(&mut self.decoder, BINCODE_CONFIG)
                .map_err(|e| format!("bincode deserialization failed: {}", e))?;

        Ok(item)
    }
}
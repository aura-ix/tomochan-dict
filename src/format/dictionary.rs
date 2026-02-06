use super::store::Store;
use super::index::DictionaryIndex;
use super::container::{ContainerFormat, Role};
use crate::schema::BINCODE_CONFIG;
use std::fs::File;
use bincode::{Encode, Decode};
use std::io::{Seek, SeekFrom};

// TODO: make sure we are not using usize in any serialized data
#[derive(Encode, Decode)]
pub struct DictionaryHeader {
    pub fst_len: u64,
    pub store_len: u64,
}

pub struct Dictionary {
    pub index: DictionaryIndex,
    pub store: Store<File>,
}

impl ContainerFormat for Dictionary {
    fn role() -> Role {
        Role::Dictionary
    }

    fn min_role_version() -> u64 {
        0
    }

    fn role_version() -> u64 {
        0
    }

    fn load(path: &str, payload_offset: u64, verify: bool) -> Result<Self, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to read package file: {}", e))?;

        file.seek(SeekFrom::Start(payload_offset))
            .map_err(|e| format!("Failed to seek to dictionary contents: {}", e))?;

        let header: DictionaryHeader = bincode::decode_from_std_read(&mut file, BINCODE_CONFIG)
            .map_err(|e| format!("Failed to decode header: {}", e))?;

        let base_offset = file.stream_position()
            .map_err(|e| format!("Failed to get stream position: {}", e))?;

        let file_meta = file.metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;

        if file_meta.len() != base_offset + header.fst_len + header.store_len {
            return Err(format!("expected file length {}, found {}", file_meta.len(), base_offset + header.fst_len + header.store_len))
        }

        // setup file for store
        file.seek(SeekFrom::Start(base_offset + header.fst_len))
            .map_err(|e| format!("Failed to seek to store contents: {}", e))?;

        let dict = Self {
            index: DictionaryIndex::load_mmap(
                path,
                base_offset,
                header.fst_len,
            )?,
            store: Store::new(file)?,
        };

        if verify {
            // TODO: verify FST
            // TODO: verify store
        }

        Ok(dict)
    }
}
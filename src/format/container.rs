//! Container format for assets usable by tomochan.
//! # Format
//! A tomochan container can be identified by the file beginning with
//! "TOMOCHAN:XXXX:", where XXXX are 4 ASCII hexadecimal (0-f/F) characters
//! which indicates the minimum supported version of the container format.
//! Version 0 is reserved for indicating a development version, which
//! is not guaranteed to work with any particular version of tomochan.
//!
//! A JSON serialization of a ContainerHeader follows the magic header in the
//! current version.

// TODO: separate out user specifiable part of container header, then the code that writes the format will file in role info, and this code will fill in container version info
// TODO: keep track of supported role versions in the code
// TODO: implement dev versions properly
// TODO: repo features
// TODO: look into validating file formats to be able to do non-validated lookups, just checking the file hash

use std::io::{self, Write, Read, Seek, SeekFrom};
use serde::{Serialize, Deserialize};
use serde_json::de::Deserializer;
use sha2::{Sha256, Digest};

const CURRENT_HEADER_VERSION: u16 = 0;
const CURRENT_CONTAINER_VERSION: u64 = 0;
const MIN_COMPATIBLE_HEADER_VERSION: u16 = 0;

#[derive(Debug, Serialize, Deserialize)]
pub enum Role {
    Dictionary,
    DeconjugationRuleset,
    // TODO: implement serde of this properly
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerHeader {
    /// Actual version of the container format
    pub container_version: u64,

    /// User visible name (ex. Jitendex)
    pub name: String,
    /// User visible revision name (ex. 2025-01-01)
    pub revision_name: String,
    /// Internal revision number, larger is always newer.
    pub revision: u64,

    /// Kind of file
    pub role: Role,
    /// Minimum suported version of the role-specific format contained inside
    pub min_role_version: u64,

    // Length of the data following the header
    pub payload_length: u64,
    // TODO: serialize as hex string
    // Hash of the data following the header
    pub payload_sha256: [u8; 32],
}

impl ContainerHeader {
    pub fn new(name: String, revision_name: String, revision: u64, role: Role, min_role_version: u64) -> Self {
        ContainerHeader {
            container_version: CURRENT_CONTAINER_VERSION,
            name: name,
            revision_name: revision_name,
            revision: revision,
            role: role,
            min_role_version: min_role_version,
            payload_length: 0,
            payload_sha256: [0u8; 32],
        }
    }
}

pub struct ContainerFileInfo {
    pub header: ContainerHeader,
    pub payload_offset: u64,
}

impl ContainerFileInfo {
    pub fn read_container<R: Read + Seek>(mut reader: R) -> io::Result<Self> {
        const MAGIC_LEN: usize = "TOMOCHAN:XXXX:".len();

        let mut magic = [0u8; MAGIC_LEN];
        reader.read_exact(&mut magic)?;
        let magic_str = std::str::from_utf8(&magic)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid magic prefix"))?;

        if !magic_str.starts_with("TOMOCHAN:") {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not recognized as a tomodict file"));
        }

        let version_hex = &magic_str["TOMOCHAN:".len()..MAGIC_LEN-1];
        let version = u8::from_str_radix(version_hex, 16)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid version in header"))?;

        if (version as u16) < MIN_COMPATIBLE_HEADER_VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Incompatible header version"));
        }

        let mut de = Deserializer::from_reader(&mut reader);
        let header = ContainerHeader::deserialize(&mut de)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("JSON parse error: {}", e)))?;

        let payload_offset = reader.stream_position()?;

        Ok(Self {header, payload_offset})
    }

    pub fn validate_payload<R: Read + Seek>(&self, mut reader: R) -> io::Result<()> {
        // TODO: stream_len method
        reader.seek(SeekFrom::Start(self.payload_offset))?;
        let eof = reader.seek(SeekFrom::End(0))?;
        let actual_length = eof
            .checked_sub(self.payload_offset)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file length"))?;

        if actual_length != self.header.payload_length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Payload length mismatch: header says {}, file has {}",
                    self.header.payload_length, actual_length
                ),
            ));
        }

        reader.seek(SeekFrom::Start(self.payload_offset))?;

        let mut hasher = Sha256::new();
        io::copy(&mut reader.take(self.header.payload_length), &mut hasher)?;
        let actual_hash = hasher.finalize();

        if &actual_hash[..] != &self.header.payload_sha256[..] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SHA256 hash of payload does not match header",
            ));
        }

        Ok(())
    }
}

// NB: the sha256 and length fields are not used here, and are instead calculated by this function
pub fn write_container<W: Write>(
    writer: &mut W,
    mut header: ContainerHeader,
    data: &[u8],
) -> io::Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash_result = hasher.finalize();

    header.payload_length = data.len() as u64;
    header.payload_sha256.copy_from_slice(&hash_result);

    let header_json = serde_json::to_vec(&header)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON serialization error: {}", e)))?;


    writer.write_all(format!("TOMOCHAN:{:04X}:", CURRENT_HEADER_VERSION).as_bytes())?;
    writer.write_all(&header_json)?;
    writer.write_all(data)?;

    Ok(())
}

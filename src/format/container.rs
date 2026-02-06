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

// TODO: license info in repo
// TODO: separate out user specifiable part of container header, then the code that writes the format will file in role info, and this code will fill in container version info
// TODO: repo features
// TODO: look into validating file formats to be able to do non-validated lookups, just checking the file hash

use std::io::{self, Write, Read, Seek, SeekFrom};
use serde::{Serialize, Deserialize};
use serde_json::de::Deserializer;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::fmt;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

const CURRENT_HEADER_VERSION: u16 = 0;
const CURRENT_CONTAINER_VERSION: u64 = 0;
const MIN_COMPATIBLE_HEADER_VERSION: u16 = 0;

static ALLOW_DEV_VERSION: AtomicBool = AtomicBool::new(false);

pub fn dev_version_allowed() -> bool {
    ALLOW_DEV_VERSION.load(Ordering::Relaxed)
}

pub fn allow_dev_version(enabled: bool) {
    ALLOW_DEV_VERSION.store(enabled, Ordering::Relaxed);
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    Dictionary,
    Deinflector,
    #[serde(untagged)]
    Unknown(String),
}

impl Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Dictionary => write!(f, "Dictionary"),
            Role::Deinflector => write!(f, "Deinflector"),
            Role::Unknown(s) => write!(f, "Unknown ({})", s),
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerMeta {
    /// User visible name (ex. Jitendex)
    pub name: String,
    /// User visible revision name (ex. 2025-01-01)
    pub revision_name: String,
    /// Internal revision number, larger is always newer.
    pub revision: u64,
}

// TODO: probably can just hide this as impl detail
#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerHeader {
    /// Actual version of the container format
    pub container_version: u64,

    #[serde(flatten)]
    pub meta: ContainerMeta,

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

pub struct OpenContainer<R: Read + Seek> {
    pub header: ContainerHeader,
    pub payload_offset: u64,
    pub stream: R,
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

pub fn write_container<T: ContainerFormat, W: Write>(
    writer: &mut W,
    meta: ContainerMeta,
    data: &[u8],
) -> io::Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let payload_sha256 = hasher.finalize();

    let header = ContainerHeader {
        container_version: CURRENT_CONTAINER_VERSION,
        meta,
        role: T::role(),
        min_role_version: T::role_version(),
        payload_length: data.len() as u64,
        payload_sha256: payload_sha256.into(),
    };


    let header_json = serde_json::to_vec(&header)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("JSON serialization error: {}", e)))?;

    writer.write_all(format!("TOMOCHAN:{:04X}:", CURRENT_HEADER_VERSION).as_bytes())?;
    writer.write_all(&header_json)?;
    writer.write_all(data)?;

    Ok(())
}

pub trait ContainerFormat: Sized {
    fn role() -> Role;
    fn min_role_version() -> u64;
    fn role_version() -> u64;

    // TODO: store verification <-> hash in .hashlog file, still use tomochan wrapper?
    // hashlog file needs to use the hash of the entire file, not the sha256 inside. maybe the sha256 needs to go at the end?
    fn load(path: &str, payload_offset: u64, verify: bool) -> Result<Self, String>;
}

// TODO: should probably expose the header somehow
pub fn open_container<T: ContainerFormat>(path: &str, verify: bool) -> Result<T, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to read package file: {}", e))?;

    let container = ContainerFileInfo::read_container(file)
        .map_err(|e| format!("Error opening container: {}", e))?;

    if container.header.role != T::role() {
        return Err(format!("container role is different than expected: {} {}", container.header.role, T::role()));
    }

    if container.header.min_role_version > T::role_version() {
        return Err("container role format too new".to_string());
    }

    if container.header.min_role_version == 0 && !dev_version_allowed() {
        return Err("package is a development version".to_string());
    }

    // TODO: sha256 when verify

    T::load(path, container.payload_offset, verify)
}
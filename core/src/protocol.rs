use postcard;
use serde::Serialize;
use serde::Deserialize;

use super::error::Error;
use super::types::Result;

// Only a maximum of 5 megabytes can be serialized at once
const MAX_MESSAGE_LENGTH: usize = 5_242_880;

// constraints defined by each device for mutual communication
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Limits {
    pub frame_size: u32,
    pub directory_count: u32,
    pub directory_depth: u8,
    pub file_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileState {
    pub filepath: String,
    pub empty: bool,
    pub bitmap: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub filepath: String,
    pub file_size: u64,
    pub chunk_count: u64,
    pub hash: [u8; 32],
    pub mtime: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EntryKind {
    Directory { path: String },
    File(FileMetadata),
}

// some bullshit
// TODO => Add better documentation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Protocol {

    // devices must agree upon mutual limits and roles 
    Hello { version: u8, sender: bool, limits: Limits },

    // the sender passes data requirements to the receiver
    ManifestBegin { root: String, dir_count: u32, dir_depth: u8, file_count: u32, total_size: u64 },

    // Per file metadata
    ManifestBatch { files: Vec<EntryKind> },

    // the sender confirms metadata quantity passed to the receiver
    ManifestEnd { files_sent: u32, dir_sent: u32 },

    // chunk hashes for all files
    ChunkHashes { filepath: String, hashes: Vec<[u8; 32]> },

    // A list of all files with incomplete bitmaps
    Want { file_state: Vec<FileState> },

    // data sent by the sender for a particular chunk
    Chunk { filepath: String, chunk_index: u64, data: Vec<u8> },

    // the receiver sends an acknowledgement for a chunk that it has received and verified
    Ack { filepath: String, chunk_index: u64 },

    // a final tally of total chunks received or sent
    Done { chunks_exchanged: u64 },

    // Error propagation between devices
    Error { code: u16, comments: String },
}

impl Protocol {
    // convert message to bytes that can be passed through the network
    pub fn to_bytes(self) -> Result<Vec<u8>> {
        let payload = postcard::to_allocvec(&self)?;
        if payload.len() > MAX_MESSAGE_LENGTH {
            return Err(Error::MessageTooLargeForFrame);
        }

        if payload.len() == 0 {
            return Err(Error::InvalidFrameLength);
        }

        let message_len = (payload.len() as u32).to_be_bytes();
        let mut frame = Vec::with_capacity(payload.len() + 4);
        frame.extend_from_slice(&message_len);
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    // convert received bytes to a valid beacon message
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Protocol> {
        if bytes.len() - 4 > MAX_MESSAGE_LENGTH {
            return Err(Error::MessageTooLargeForFrame);
        }

        if bytes.len() < 4 {
            return Err(Error::InvalidFrameLength);
        }

        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let payload = &bytes[4..];

        if payload.len() != len {
            return Err(Error::FramelengthMistmatch);
        }

        let message: Protocol = postcard::from_bytes(payload)?;
        Ok(message)
    }
}
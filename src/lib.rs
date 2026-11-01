

mod record;
mod config;
mod error;

use std::{fs::File, path::Path};
use fslock::LockFile;

use record::{SeqNo, Record};
use config::QueueConfig;
pub struct Queue {
    _lock: LockFile,
    acked: SeqNo,
    next_seq: SeqNo,
}

impl Queue {
    pub fn open(dir: &Path, cfg: QueueConfig) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let lock_file_path = dir.join(".pigeon.lock");
        let mut lock = LockFile::open(&lock_file_path)?;
        lock.lock()?;

        let acked = read_ack_watermark(dir);

        

        todo!()
    }

    pub async fn enqueue(&self, payload: &[u8]) -> Result<SeqNo, error::EnqueueError> {
        todo!()
    }
    pub async fn next_batch(&self, max: usize) -> Vec<Record> {
        todo!()
    }
    pub fn ack(&self, upto: SeqNo) -> std::io::Result<()> {
        todo!()
    }
    pub fn stats(&self) -> QueueStats {
        todo!()
    }
}

const ACK_FILE: &str = "ack.chk";
const ACK_LEN: usize = 12;

fn encode_ack(upto: SeqNo) -> [u8; ACK_LEN] {
    let seq = upto.to_le_bytes();
    let crc = crc32fast::hash(&seq);
    let mut out = [0u8; ACK_LEN];
    out[..8].copy_from_slice(&seq);
    out[8..].copy_from_slice(&crc.to_le_bytes());
    out
}

fn read_ack_watermark(dir: &Path) -> std::io::Result<SeqNo> {
    let bytes = match std::fs::read(dir.join(ACK_FILE)) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };

    if bytes.len() != ACK_LEN {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "ack.chk has wrong length"));
    }

    let seq: [u8; 8] = bytes[..8].try_into().expect("checked length");
    let stored_crc = u32::from_le_bytes(bytes[8..].try_into().expect("checked length"));

    if crc32fast::hash(&seq) != stored_crc {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "ack.chk checksum mismatch"));
    }

    Ok(u64::from_le_bytes(seq))
}

pub struct QueueStats { }

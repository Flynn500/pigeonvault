mod config;
mod error;
mod record;

use fslock::LockFile;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use config::QueueConfig;
use record::{Record, SeqNo, scan_records};

const SEG_MAGIC: [u8; 4] = *b"PGSG";
const SEG_VERSION: u16 = 1;
const SEG_HEADER_LEN: usize = 20;

#[derive(Debug, Clone)]
pub struct SegMeta {
    pub base_seq: SeqNo,
    pub path: PathBuf,
}

struct SegHeader {
    base_seq: SeqNo,
}

pub struct QueueStats {}

pub struct Queue {
    _lock: LockFile,
    acked: SeqNo,
    next_seq: SeqNo,
    segments: Vec<SegMeta>,
}

impl Queue {
    pub fn open(dir: &Path, cfg: QueueConfig) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let mut lock = LockFile::open(&dir.join(".pigeon.lock"))?;
        lock.lock()?;

        let acked = read_ack_watermark(dir)?;

        let mut segments = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some("seg") => {
                    let header = read_seg_header(&path)?;
                    segments.push(SegMeta {
                        base_seq: header.base_seq,
                        path,
                    });
                }
                Some("tmp") => std::fs::remove_file(&path)?,
                _ => {}
            }
        }

        segments.sort_by_key(|s| s.base_seq);
        for pair in segments.windows(2) {
            if pair[0].base_seq == pair[1].base_seq {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "duplicate base_seq {} in {:?} and {:?}",
                        pair[0].base_seq, pair[0].path, pair[1].path
                    ),
                ));
            }
        }

        let next_seq = match segments.last() {
            Some(last) => {
                let bytes = std::fs::read(&last.path)?;
                let scan = scan_records(&bytes[SEG_HEADER_LEN..], last.base_seq);
                if scan.tail.is_some() {
                    truncate_tail(&last.path, (SEG_HEADER_LEN + scan.valid_len) as u64)?;
                }
                scan.next_seq
            }
            None => acked,
        };

        if acked > next_seq {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("ack watermark {acked} is past next_seq {next_seq}"),
            ));
        }

        Ok(Self {
            _lock: lock,
            acked,
            next_seq,
            segments,
        })
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
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "ack.chk has wrong length",
        ));
    }

    let seq: [u8; 8] = bytes[..8].try_into().expect("checked length");
    let stored_crc = u32::from_le_bytes(bytes[8..].try_into().expect("checked length"));

    if crc32fast::hash(&seq) != stored_crc {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "ack.chk checksum mismatch",
        ));
    }

    Ok(u64::from_le_bytes(seq))
}

fn read_seg_header(path: &Path) -> std::io::Result<SegHeader> {
    let mut buf = [0u8; SEG_HEADER_LEN];
    File::open(path)?.read_exact(&mut buf)?;

    let magic: [u8; 4] = buf[0..4].try_into().expect("fixed slice");
    let version = u16::from_le_bytes(buf[4..6].try_into().expect("fixed slice"));
    let reserved = u16::from_le_bytes(buf[6..8].try_into().expect("fixed slice"));
    let base_seq = u64::from_le_bytes(buf[8..16].try_into().expect("fixed slice"));
    let stored_crc = u32::from_le_bytes(buf[16..20].try_into().expect("fixed slice"));

    let invalid = |msg: String| std::io::Error::new(std::io::ErrorKind::InvalidData, msg);

    if magic != SEG_MAGIC {
        return Err(invalid(format!("{path:?}: bad magic {magic:?}")));
    }
    if version != SEG_VERSION {
        return Err(invalid(format!("{path:?}: unsupported version {version}")));
    }
    if reserved != 0 {
        return Err(invalid(format!("{path:?}: reserved field is nonzero")));
    }

    if crc32fast::hash(&buf[..16]) != stored_crc {
        return Err(invalid(format!("{path:?}: header checksum mismatch")));
    }

    Ok(SegHeader { base_seq })
}

fn truncate_tail(path: &Path, valid_end: u64) -> std::io::Result<()> {
    let f = std::fs::OpenOptions::new().write(true).open(path)?;
    if f.metadata()?.len() > valid_end {
        f.set_len(valid_end)?;
        f.sync_all()?;
    }
    Ok(())
}

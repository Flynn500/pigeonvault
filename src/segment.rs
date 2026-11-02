use crate::record::SeqNo;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub const SEG_MAGIC: [u8; 4] = *b"PGSG";
pub const SEG_VERSION: u16 = 1;
pub const SEG_HEADER_LEN: usize = 20;

#[derive(Debug, Clone)]
pub struct SegMeta {
    pub base_seq: SeqNo,
    pub path: PathBuf,
}

pub struct SegHeader {
    pub base_seq: SeqNo,
}

pub(crate) fn read_seg_header(path: &Path) -> std::io::Result<SegHeader> {
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

/* 
                    Record encoding

| len: u32 | seq: u64 | crc: u32 | payload: len bytes  |
   4 bytes    8 bytes    4 bytes     variable

*/

use crc32fast::Hasher;

pub type SeqNo = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub seq: SeqNo,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum DecodeError {
    Truncated,
    TooLarge(u32),
    BadChecksum,
}

#[derive(Debug)]
pub enum EncodeError {
    PayloadTooLarge { len: usize, max: usize },
}

pub const MAX_PAYLOAD: usize = 256 * 1024;
const HEADER_LEN: usize = 16;

fn checksum(len: u32, seq: SeqNo, payload: &[u8]) -> u32 {
    let mut h = Hasher::new();
    h.update(&len.to_le_bytes());
    h.update(&seq.to_le_bytes());
    h.update(payload);
    h.finalize()
}

impl Record {
    pub fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        if self.payload.len() > MAX_PAYLOAD {
            return Err(EncodeError::PayloadTooLarge {
                len: self.payload.len(),
                max: MAX_PAYLOAD,
            });
        }

        let len = self.payload.len() as u32;
        let crc = checksum(len, self.seq, &self.payload);

        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&self.seq.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&self.payload);
        Ok(())
    }

    pub fn decode(buf: &[u8]) -> Result<(Record, usize), DecodeError> {
        if buf.len() < HEADER_LEN {
            return Err(DecodeError::Truncated);
        }

        let len = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let seq = u64::from_le_bytes(buf[4..12].try_into().unwrap());
        let expected_crc = u32::from_le_bytes(buf[12..16].try_into().unwrap());

        if len as usize > MAX_PAYLOAD {
            return Err(DecodeError::TooLarge(len));
        }

        let frame_len = HEADER_LEN + len as usize;
        if buf.len() < frame_len {
            return Err(DecodeError::Truncated);
        }

        let payload = &buf[HEADER_LEN..frame_len];
        if checksum(len, seq, payload) != expected_crc {
            return Err(DecodeError::BadChecksum);
        }

        Ok((Record { seq, payload: payload.to_vec() }, frame_len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_record() {
        let cases: &[&[u8]] = &[
            b"",
            b"hello mqtt",
            &[0xFF; 1024],
        ];

        for &payload in cases {
            let original = Record {
                seq: 2312,
                payload: payload.to_vec(),
            };

            let mut buf = Vec::new();
            original.encode_into(&mut buf).unwrap();

            let (decoded, consumed) = Record::decode(&buf).unwrap();

            assert_eq!(decoded, original);
            assert_eq!(consumed, buf.len());
            assert_eq!(consumed, HEADER_LEN + payload.len());
        }
    }
}
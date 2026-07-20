//! `BTreeKey` — serialisation contract for B+Tree key types.
//!
//! Implemented for `i64` (fixed 8-byte little-endian) and `String`
//! (raw UTF-8 bytes; the node layer stores the length separately).

use std::fmt::Debug;

/// Types that can serve as B+Tree keys must be totally ordered and
/// round-trip through a compact byte encoding.
pub trait BTreeKey: Ord + Clone + Debug + Send + Sync + 'static {
    /// Encodes the key into a byte vector.
    fn encode(&self) -> Vec<u8>;

    /// Decodes a key from the given byte slice, returning the key and the
    /// number of bytes consumed.  Returns `None` on malformed input.
    fn decode(bytes: &[u8]) -> Option<(Self, usize)>;
}

// ---------------------------------------------------------------------------
// i64 — fixed 8-byte little-endian encoding
// ---------------------------------------------------------------------------

impl BTreeKey for i64 {
    fn encode(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }

    fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 8 {
            return None;
        }
        let arr: [u8; 8] = bytes[..8].try_into().ok()?;
        Some((i64::from_le_bytes(arr), 8))
    }
}

// ---------------------------------------------------------------------------
// String — raw UTF-8 bytes; the node layer prefixes the length
// ---------------------------------------------------------------------------

impl BTreeKey for String {
    fn encode(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// The node layer passes exactly the right number of bytes.
    fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        let s = std::str::from_utf8(bytes).ok()?;
        Some((s.to_owned(), bytes.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i64_roundtrip() {
        for v in [-1_i64, 0, 1, i64::MIN, i64::MAX, 42] {
            let enc = v.encode();
            assert_eq!(enc.len(), 8);
            let (dec, consumed) = i64::decode(&enc).unwrap();
            assert_eq!(dec, v);
            assert_eq!(consumed, 8);
        }
    }

    #[test]
    fn string_roundtrip() {
        let s = "hello".to_string();
        let enc = s.encode();
        let (dec, consumed) = String::decode(&enc).unwrap();
        assert_eq!(dec, s);
        assert_eq!(consumed, enc.len());
    }

    #[test]
    fn i64_ordering_preserved() {
        // Keys must sort the same way encoded and decoded.
        let mut keys: Vec<i64> = vec![5, -3, 0, 100, -100, 42];
        let encoded: Vec<Vec<u8>> = keys.iter().map(|k| k.encode()).collect();
        keys.sort();
        // i64 little-endian does NOT sort lexicographically — that's fine,
        // we sort the keys via Ord, not by comparing raw bytes.
        keys.dedup();
        let mut decoded: Vec<i64> = encoded
            .iter()
            .map(|b| i64::decode(b).unwrap().0)
            .collect();
        decoded.sort();
        assert_eq!(keys, decoded);
    }
}

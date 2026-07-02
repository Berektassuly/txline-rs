use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde::de::value::{Error as ValueError, SeqDeserializer};
use serde_json::json;
use txline::validation::{Hash32, ProofNode};

#[test]
fn decodes_base64_hash_to_32_bytes() {
    let bytes = [7u8; 32];
    let encoded = STANDARD.encode(bytes);
    let hash = Hash32::decode(&encoded).unwrap();
    assert_eq!(hash.as_bytes(), &bytes);
}

#[test]
fn decodes_hex_hash_to_32_bytes() {
    let hash = Hash32::decode(&format!("0x{}", "ab".repeat(32))).unwrap();
    assert_eq!(hash.as_bytes(), &[0xabu8; 32]);
}

#[test]
fn deserializes_array_hash_to_32_bytes() {
    let node: ProofNode =
        serde_json::from_value(json!({ "hash": vec![3u8; 32], "isRightSibling": true })).unwrap();
    assert_eq!(node.hash.as_bytes(), &[3u8; 32]);
    assert!(node.is_right_sibling);
}

#[test]
fn rejects_wrong_length_hash() {
    let err = Hash32::from_bytes([1u8; 31]).unwrap_err();
    assert!(err.to_string().contains("expected 32 bytes"));
}

#[test]
fn rejects_oversized_array_hash_after_thirty_two_bytes() {
    let seq = SeqDeserializer::<_, ValueError>::new(PanicAfterThirtyThreeBytes { yielded: 0 });

    let err = Hash32::deserialize(seq).unwrap_err();

    assert!(err.to_string().contains("more than 32"));
}

struct PanicAfterThirtyThreeBytes {
    yielded: usize,
}

impl Iterator for PanicAfterThirtyThreeBytes {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.yielded += 1;
        assert!(
            self.yielded <= 33,
            "Hash32 deserializer read beyond the first oversized byte"
        );
        Some(7)
    }
}

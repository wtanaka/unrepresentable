use super::{decode, encode, Bytes};
use crate::types::Text;

#[test]
fn known_vectors_encode_with_padding() {
    for (raw, wanted) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(
            encode(&Bytes::of(raw.as_bytes())).as_str(),
            wanted,
            "encoding of {raw:?}"
        );
        assert_eq!(
            decode(&Text::of(wanted)),
            Ok(Bytes::of(raw.as_bytes())),
            "decoding of {wanted:?}"
        );
    }
}

#[test]
fn every_byte_value_round_trips() {
    let everything: Vec<u8> = (0..=u8::MAX).collect();
    let encoded = encode(&Bytes::from_vec(everything.clone()));
    assert_eq!(
        decode(&encoded),
        Ok(Bytes::from_vec(everything)),
        "all 256 byte values"
    );
}

#[test]
fn decoding_is_strict() {
    for bad in [
        "Zg=", "Zg", "Zg===", "Z=g=", "Zm9v\n", " Zm9v", "Zm9v-_", "Zh==", "====", "Zm9vY",
    ] {
        assert!(
            decode(&Text::of(bad)).is_err(),
            "{bad:?} must be refused: wrong length, padding, alphabet or trailing bits"
        );
    }
}

use ocr2md_core::secure_config::{decrypt_blob, encrypt_blob};

#[test]
fn encrypt_decrypt_roundtrip() {
    let plain = br#"{"profiles":[{"name":"openai","api_key":"secret"}]}"#;
    let cipher = encrypt_blob(plain, "passphrase").unwrap();
    let back = decrypt_blob(&cipher, "passphrase").unwrap();
    assert_eq!(back, plain);
}

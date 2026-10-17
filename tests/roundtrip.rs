//! Integration tests that use only the public API, the way a library user would.

use huffman_cli::error::HuffmanError;
use huffman_cli::huffman::{decode_from_bytes, decode_to_file, encode_to_bytes, encode_to_file};
use std::fs;

/// Builds a temp path that is unique per process, because tests run in
/// parallel and the library refuses to overwrite files.
fn temp(name: &str) -> String {
    let file = format!("{}_{}", std::process::id(), name);
    std::env::temp_dir()
        .join(file)
        .to_str()
        .unwrap()
        .to_string()
}

fn file_round_trip(tag: &str, text: &str) {
    let enc = temp(&format!("{}.en.huff", tag));
    let dec = temp(&format!("{}.dec.txt", tag));
    let _ = fs::remove_file(&enc);
    let _ = fs::remove_file(&dec);

    encode_to_file(text, &enc).unwrap();
    decode_to_file(&enc, &dec).unwrap();
    assert_eq!(fs::read_to_string(&dec).unwrap(), text);

    fs::remove_file(&enc).unwrap();
    fs::remove_file(&dec).unwrap();
}

#[test]
fn round_trip_abracadabra() {
    file_round_trip("abra", "abracadabra");
}

#[test]
fn round_trip_single_char() {
    file_round_trip("single", "aaaa");
}

#[test]
fn round_trip_unicode_and_newlines() {
    file_round_trip("uni", "नमस्ते 😀\nline two\r\n");
}

#[test]
fn round_trip_long_text() {
    let text = "the quick brown fox jumps over the lazy dog\n".repeat(500);
    file_round_trip("long", &text);
}

#[test]
fn long_text_actually_shrinks() {
    let text = "the quick brown fox jumps over the lazy dog\n".repeat(500);
    let packed = encode_to_bytes(&text).unwrap();
    assert!(packed.len() < text.len());
    assert_eq!(decode_from_bytes(&packed).unwrap(), text);
}

#[test]
fn decode_refuses_to_overwrite() {
    let enc = temp("noover.en.huff");
    let dec = temp("noover.dec.txt");
    let _ = fs::remove_file(&enc);
    let _ = fs::remove_file(&dec);

    encode_to_file("hello", &enc).unwrap();
    fs::write(&dec, "existing").unwrap();

    let result = decode_to_file(&enc, &dec);
    assert!(matches!(result, Err(HuffmanError::OutputExists(_))));
    assert_eq!(fs::read_to_string(&dec).unwrap(), "existing");

    fs::remove_file(&enc).unwrap();
    fs::remove_file(&dec).unwrap();
}

#[test]
fn missing_input_is_io_error() {
    let out = temp("missing.dec.txt");
    let result = decode_to_file("this_file_does_not_exist.en.huff", &out);
    assert!(matches!(result, Err(HuffmanError::Io(_))));
}

#[test]
fn truncated_file_is_corrupted() {
    let enc = temp("trunc.en.huff");
    let cut = temp("trunc_cut.en.huff");
    let out = temp("trunc.dec.txt");
    for p in [&enc, &cut, &out] {
        let _ = fs::remove_file(p);
    }

    encode_to_file("abracadabra", &enc).unwrap();
    let data = fs::read(&enc).unwrap();
    fs::write(&cut, &data[..data.len() - 1]).unwrap();

    let result = decode_to_file(&cut, &out);
    assert!(matches!(result, Err(HuffmanError::CorruptedData)));

    fs::remove_file(&enc).unwrap();
    fs::remove_file(&cut).unwrap();
}

#[test]
fn tiny_garbage_file_is_corrupted() {
    let bad = temp("garbage.en.huff");
    let out = temp("garbage.dec.txt");
    let _ = fs::remove_file(&bad);
    fs::write(&bad, [1, 2, 3]).unwrap();

    let result = decode_to_file(&bad, &out);
    assert!(matches!(result, Err(HuffmanError::CorruptedData)));

    fs::remove_file(&bad).unwrap();
}

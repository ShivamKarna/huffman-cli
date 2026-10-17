//! Huffman coding for UTF-8 text.
//!
//! This crate compresses text with Huffman coding and decompresses it again.
//! It ships as a library and as the `huff` command line tool.
//!
//! # Quick start
//!
//! Compress and decompress in memory:
//!
//! ```
//! use huffman_cli::huffman::{decode_from_bytes, encode_to_bytes};
//!
//! let packed = encode_to_bytes("abracadabra").unwrap();
//! let text = decode_from_bytes(&packed).unwrap();
//! assert_eq!(text, "abracadabra");
//! ```
//!
//! Compress and decompress files:
//!
//! ```no_run
//! use huffman_cli::huffman::{decode_to_file, encode_to_file};
//!
//! encode_to_file("some text to compress", "out.en.huff").unwrap();
//! decode_to_file("out.en.huff", "out.dec.txt").unwrap();
//! ```
//!
//! # Encoded format
//!
//! All integers are little-endian.
//!
//! | Field       | Size                | Description                               |
//! |-------------|---------------------|-------------------------------------------|
//! | char count  | 8 bytes (`u64`)     | Number of characters in the original text |
//! | tree length | 4 bytes (`u32`)     | Size of the serialized tree in bytes      |
//! | tree        | `tree length` bytes | Pre-order encoding of the Huffman tree    |
//! | packed bits | remaining bytes     | Encoded text, most significant bit first  |
//!
//! A tree node is written in pre-order. An internal node is the byte `0`
//! followed by its left and right subtrees. A leaf is the byte `1` followed by
//! the character as a 4 byte Unicode scalar value.
//!
//! The last byte of the packed bits is padded with zero bits. The character
//! count tells the decoder where the real data ends.
//!
//! # Errors
//!
//! Every fallible function returns [`error::HuffmanError`]. Malformed input is
//! reported as an error and never causes a panic.
//!
//! # Limitations
//!
//! * Input must be valid UTF-8 text.
//! * Data is processed in memory, with no streaming.
//! * The format is specific to this crate and may change before version 1.0.

pub mod error;
pub mod file_helpers;
pub mod huffman;

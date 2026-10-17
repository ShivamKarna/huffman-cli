# huffman-cli

A command line tool that compresses and decompresses UTF-8 text files using Huffman coding.

This is a small, self-contained implementation intended for learning and for use as a reference. It is not a replacement for general-purpose compressors such as gzip or zstd.

## Installation

From crates.io:

```
cargo install huffman-cli
```

From source:

```
git clone https://github.com/shivamkarna/huffman-cli.git
cd huffman-cli
cargo build --release
```

The binary is written to `target/release/huff`.

## Usage

Compress a text file:

```
huff -en notes.txt
```

This writes `notes.en.huff` next to the input.

Decompress:

```
huff -den notes.en.huff
```

This writes `notes.dec.txt` next to the input.

| Flag   | Input extension | Output extension |
| ------ | --------------- | ---------------- |
| `-en`  | `.txt`          | `.en.huff`       |
| `-den` | `.en.huff`      | `.dec.txt`       |

The tool never overwrites an existing file. If the output path already exists, it exits with an error and leaves both files untouched. Delete or move the existing file and run the command again.

Errors are written to stderr and the process exits with status 1. Where an underlying cause exists (for example a missing input file), it is printed on a following line.

## How it works

1. Count how often each Unicode character occurs in the input.
2. Build a Huffman tree with a min-heap, so frequent characters get short codes.
3. Derive a code for each character from its path in the tree (left is `0`, right is `1`).
4. Serialize the tree and pack the encoded bits into bytes.
5. To decode, read the tree from the file and walk it one bit at a time, emitting a character at each leaf and returning to the root.

The tree is stored in the output file, so the decoder never recomputes frequencies and does not depend on how ties were broken during encoding. Two runs on the same input may produce different trees, and both decode correctly.

## File format

All integers are little-endian.

| Field       | Size                | Description                               |
| ----------- | ------------------- | ----------------------------------------- |
| char count  | 8 bytes (`u64`)     | Number of characters in the original text |
| tree length | 4 bytes (`u32`)     | Size of the serialized tree in bytes      |
| tree        | `tree length` bytes | Pre-order encoding of the tree            |
| packed bits | remaining bytes     | Encoded text, most significant bit first  |

Tree encoding, in pre-order (node, left, right):

- Internal node: one byte `0`, followed by the left and right subtrees.
- Leaf: one byte `1`, followed by the character as a 4 byte Unicode scalar value.

The final byte of the packed bits is padded with zero bits. The decoder uses the character count to know where the real data ends, so padding is never decoded.

A file containing a single distinct character has a tree consisting of one leaf. In that case the decoder repeats the character `char count` times and reads no bits.

## Limitations

- **Small files get larger.** The header and tree cost 12 bytes plus 5 bytes per distinct character and 1 byte per internal node. For example, `abracadabra` (11 bytes) encodes to 44 bytes. Compression only pays off once the text is long enough for the savings on the packed bits to exceed that overhead.
- **Text only.** Input must be valid UTF-8. Other files are rejected with an error rather than processed as raw bytes.
- **Symbols are characters, not bytes.** Frequencies are counted per Unicode scalar value, which suits text but means a file with a very large alphabet carries a large tree.
- **In-memory processing.** Files are read and written whole. There is no streaming, so very large inputs are limited by available memory.
- **No integrity check.** The decoder detects truncated or structurally invalid files, but it does not store a checksum, so corruption that still yields a valid bit stream will not be reported.
- **Format stability.** The format is specific to this tool and is not compatible with any other Huffman implementation. It may change before version 1.0.

## Error handling

Malformed input is reported as an error and never causes a panic. The decoder returns a corrupted data error for any of the following:

- A file shorter than the 12 byte header.
- A tree length that extends past the end of the file.
- An unknown node tag, a truncated leaf, or an invalid Unicode value in the tree.
- A bit sequence that leads to a path not present in the tree.
- A bit stream that ends before the declared number of characters has been decoded.

## Development

Run the full test suite:

```
cargo test
```

The suite contains unit tests for the individual stages (counting, tree building, code generation, serialization, bit packing, decoding), integration tests that encode and decode real files through the public API, and end to end tests that run the compiled binary.

Before submitting changes, run:

```
cargo fmt
cargo clippy
cargo test
```

## License

Licensed under either of MIT or Apache-2.0, at your option.

## License

Licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE)
- MIT license (LICENSE-MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

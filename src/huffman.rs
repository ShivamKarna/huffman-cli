//! The Huffman encoder and decoder.
//!
//! The public entry points are [`encode_to_bytes`], [`decode_from_bytes`],
//! [`encode_to_file`] and [`decode_to_file`]. Everything else is an
//! implementation detail.
//!
//! Encoded format: `[char_count: 8][tree_length: 4][tree][packed_bits]`.
//! See the crate level documentation for the full description.

use crate::error::HuffmanError;
use crate::file_helpers::write_new_file;
use std::collections::{BinaryHeap, HashMap};

/// Size of the fixed header: 8 bytes of character count and 4 bytes of tree length.
const HEADER_LEN: usize = 12;

/// Deepest tree the decoder accepts. A genuine Huffman tree over fewer than
/// 2^64 characters is far shallower than this, so only crafted input hits it.
/// The limit keeps recursion bounded.
const MAX_TREE_DEPTH: usize = 256;

/// A node of the Huffman tree. Leaves carry a character, internal nodes do not.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Node {
    pub(crate) ch: Option<char>,
    pub(crate) freq: u64,
    pub(crate) left: Option<Box<Node>>,
    pub(crate) right: Option<Box<Node>>,
}

impl Node {
    fn leaf(ch: Option<char>, freq: u64) -> Self {
        Self {
            ch,
            freq,
            left: None,
            right: None,
        }
    }

    fn internal(freq: u64, left: Node, right: Node) -> Self {
        Self {
            ch: None,
            freq,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

// `BinaryHeap` is a max-heap, so the comparison is reversed to pop the
// smallest frequency first.
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.freq.cmp(&self.freq)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Tree and code construction

/// Counts how many times each character occurs in `text`.
pub(crate) fn count_freq(text: &str) -> HashMap<char, usize> {
    let mut counts = HashMap::<char, usize>::new();
    for ch in text.chars() {
        *counts.entry(ch).or_insert(0) += 1;
    }
    counts
}

/// Builds the Huffman tree for the given character counts.
///
/// Returns `None` when `counts` is empty. Ties between equal frequencies are
/// broken arbitrarily, so the tree shape can differ between runs. That is fine
/// because the tree is stored in the output.
pub(crate) fn build_tree(counts: &HashMap<char, usize>) -> Option<Node> {
    if counts.is_empty() {
        return None;
    }

    let mut heap = BinaryHeap::new();

    for (&ch, &freq) in counts {
        heap.push(Node::leaf(Some(ch), freq as u64));
    }

    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();
        let freq = left.freq + right.freq;

        heap.push(Node::internal(freq, left, right));
    }

    heap.pop()
}

/// Walks the tree and records the code of every leaf. Left appends `0` and
/// right appends `1`.
fn make_codes(node: &Node, prefix: String, codes: &mut HashMap<char, String>) {
    if let Some(ch) = node.ch {
        // A tree with a single leaf has an empty prefix. Give it the code "0".
        if prefix.is_empty() {
            codes.insert(ch, "0".into());
        } else {
            codes.insert(ch, prefix);
        }
        return;
    }

    if let Some(left_node) = node.left.as_ref() {
        make_codes(left_node, format!("{}0", prefix), codes);
    }
    if let Some(right_node) = node.right.as_ref() {
        make_codes(right_node, format!("{}1", prefix), codes);
    }
}

/// Returns the bit string (as text of `0` and `1`) for every character in the tree.
pub(crate) fn generate_code(root: &Node) -> HashMap<char, String> {
    let mut codes = HashMap::<char, String>::new();
    make_codes(root, String::new(), &mut codes);
    codes
}

// Encoding

/// Appends the tree to `out_buffer` in pre-order.
///
/// An internal node is the byte `0` followed by its children. A leaf is the
/// byte `1` followed by the character as a little-endian `u32`.
pub(crate) fn serialize_tree(node: &Node, out_buffer: &mut Vec<u8>) {
    if let Some(ch) = node.ch {
        out_buffer.push(1);
        out_buffer.extend_from_slice(&(ch as u32).to_le_bytes());
    } else {
        out_buffer.push(0);
        if let Some(left) = node.left.as_ref() {
            serialize_tree(left, out_buffer);
        }
        if let Some(right) = node.right.as_ref() {
            serialize_tree(right, out_buffer);
        }
    }
}

/// Replaces each character of `content` with its code and packs the bits into
/// bytes, most significant bit first. The last byte is padded with zero bits.
pub(crate) fn pack_bits(content: &str, codes: HashMap<char, String>) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut current: u8 = 0;
    let mut bit_count: u8 = 0;

    for ch in content.chars() {
        for bit in codes[&ch].chars() {
            current <<= 1;
            if bit == '1' {
                current |= 1;
            }
            bit_count += 1;

            if bit_count == 8 {
                bytes.push(current);
                current = 0;
                bit_count = 0;
            }
        }
    }

    if bit_count > 0 {
        bytes.push(current << (8 - bit_count));
    }
    bytes
}

/// Compresses `content` and returns the encoded bytes.
///
/// The result contains everything needed to decode it again, including the
/// Huffman tree, so it can be passed straight to [`decode_from_bytes`].
///
/// # Errors
///
/// Returns [`HuffmanError::EmptyFile`] if `content` is empty.
///
/// # Examples
///
/// ```
/// use huffman_cli::huffman::{decode_from_bytes, encode_to_bytes};
///
/// let packed = encode_to_bytes("hello world").unwrap();
/// assert_eq!(decode_from_bytes(&packed).unwrap(), "hello world");
/// ```
///
/// Empty input is rejected:
///
/// ```
/// use huffman_cli::error::HuffmanError;
/// use huffman_cli::huffman::encode_to_bytes;
///
/// assert!(matches!(encode_to_bytes(""), Err(HuffmanError::EmptyFile)));
/// ```
pub fn encode_to_bytes(content: &str) -> Result<Vec<u8>, HuffmanError> {
    let counts = count_freq(content);
    let root = build_tree(&counts).ok_or(HuffmanError::EmptyFile)?;
    let codes = generate_code(&root);

    let mut tree_bytes = Vec::new();
    serialize_tree(&root, &mut tree_bytes);

    let packed = pack_bits(content, codes);
    let total_chars = content.chars().count() as u64;

    let mut out_buffer = Vec::with_capacity(HEADER_LEN + tree_bytes.len() + packed.len());
    out_buffer.extend_from_slice(&total_chars.to_le_bytes());
    out_buffer.extend_from_slice(&(tree_bytes.len() as u32).to_le_bytes());
    out_buffer.extend_from_slice(&tree_bytes);
    out_buffer.extend_from_slice(&packed);

    Ok(out_buffer)
}

/// Compresses `content` and writes the result to a new file at `output_path`.
///
/// An existing file is never overwritten.
///
/// # Errors
///
/// * [`HuffmanError::EmptyFile`] if `content` is empty. No file is created.
/// * [`HuffmanError::OutputExists`] if `output_path` already exists.
/// * [`HuffmanError::Io`] if the file cannot be written.
pub fn encode_to_file(content: &str, output_path: &str) -> Result<(), HuffmanError> {
    let encoded = encode_to_bytes(content)?;
    write_new_file(output_path, &encoded)
}

// Decoding

/// Rebuilds a tree from the bytes written by [`serialize_tree`].
///
/// `pos` is the index of the next unread byte. It is advanced as bytes are
/// consumed, so recursive calls continue where the previous one stopped.
pub(crate) fn deserialize_tree(data: &[u8], pos: &mut usize) -> Result<Node, HuffmanError> {
    deserialize_node(data, pos, 0)
}

fn deserialize_node(data: &[u8], pos: &mut usize, depth: usize) -> Result<Node, HuffmanError> {
    if depth > MAX_TREE_DEPTH || *pos >= data.len() {
        return Err(HuffmanError::CorruptedData);
    }
    let tag = data[*pos];
    *pos += 1;

    match tag {
        // Leaf: 4 bytes holding a Unicode scalar value.
        1 => {
            let end = pos.checked_add(4).ok_or(HuffmanError::CorruptedData)?;
            let bytes_slice = data.get(*pos..end).ok_or(HuffmanError::CorruptedData)?;
            let bytes = bytes_slice
                .try_into()
                .map_err(|_| HuffmanError::CorruptedData)?;

            *pos = end;
            let num = u32::from_le_bytes(bytes);
            let ch = char::from_u32(num).ok_or(HuffmanError::CorruptedData)?;
            Ok(Node::leaf(Some(ch), 0))
        }
        // Internal node: a left subtree followed by a right subtree.
        0 => {
            let left = deserialize_node(data, pos, depth + 1)?;
            let right = deserialize_node(data, pos, depth + 1)?;
            Ok(Node::internal(0, left, right))
        }
        _ => Err(HuffmanError::CorruptedData),
    }
}

/// Reads `bytes` one bit at a time and walks the tree. A `0` bit goes left and
/// a `1` bit goes right. On reaching a leaf the character is written and the
/// walk restarts at the root. Decoding stops after `total` characters, which
/// is how the zero padding in the last byte is ignored.
pub(crate) fn decode_bits(root: &Node, bytes: &[u8], total: u64) -> Result<String, HuffmanError> {
    let mut string_buf = String::new();

    // A root that is a leaf means the text had one distinct character.
    // There are no bits to read, only a repeat count.
    if let Some(ch) = root.ch {
        for _ in 0..total {
            string_buf.push(ch);
        }
        return Ok(string_buf);
    }

    let mut node = root;
    let mut count: u64 = 0;

    for byte in bytes {
        for i in (0..8).rev() {
            if count == total {
                return Ok(string_buf);
            }

            let bit = (*byte >> i) & 1;
            let child = if bit == 0 { &node.left } else { &node.right };

            node = child.as_ref().ok_or(HuffmanError::CorruptedData)?;

            if let Some(ch) = node.ch {
                string_buf.push(ch);
                count += 1;
                node = root;
            }
        }
    }

    if count != total {
        return Err(HuffmanError::CorruptedData);
    }
    Ok(string_buf)
}

/// Decompresses bytes produced by [`encode_to_bytes`].
///
/// # Errors
///
/// Returns [`HuffmanError::CorruptedData`] if the data is shorter than the
/// header, declares a tree that does not fit, contains an invalid tree, or
/// ends before the declared number of characters has been decoded.
///
/// # Examples
///
/// ```
/// use huffman_cli::huffman::{decode_from_bytes, encode_to_bytes};
///
/// let packed = encode_to_bytes("abracadabra").unwrap();
/// assert_eq!(decode_from_bytes(&packed).unwrap(), "abracadabra");
/// ```
///
/// Damaged data is reported as an error:
///
/// ```
/// use huffman_cli::error::HuffmanError;
/// use huffman_cli::huffman::decode_from_bytes;
///
/// let result = decode_from_bytes(&[1, 2, 3]);
/// assert!(matches!(result, Err(HuffmanError::CorruptedData)));
/// ```
pub fn decode_from_bytes(data: &[u8]) -> Result<String, HuffmanError> {
    if data.len() < HEADER_LEN {
        return Err(HuffmanError::CorruptedData);
    }

    let total = u64::from_le_bytes(
        data[0..8]
            .try_into()
            .map_err(|_| HuffmanError::CorruptedData)?,
    );
    let tree_len = u32::from_le_bytes(
        data[8..12]
            .try_into()
            .map_err(|_| HuffmanError::CorruptedData)?,
    ) as usize;

    let tree_end = HEADER_LEN
        .checked_add(tree_len)
        .filter(|&end| end <= data.len())
        .ok_or(HuffmanError::CorruptedData)?;

    let mut pos = HEADER_LEN;
    let root = deserialize_tree(&data[..tree_end], &mut pos)?;

    // The tree must fill exactly the space the header promised.
    if pos != tree_end {
        return Err(HuffmanError::CorruptedData);
    }

    decode_bits(&root, &data[tree_end..], total)
}

/// Reads the compressed file at `input_path` and writes the original text to a
/// new file at `output_path`.
///
/// An existing file is never overwritten.
///
/// # Errors
///
/// * [`HuffmanError::Io`] if the input cannot be read or the output cannot be written.
/// * [`HuffmanError::CorruptedData`] if the input is not valid encoded data.
/// * [`HuffmanError::OutputExists`] if `output_path` already exists.
pub fn decode_to_file(input_path: &str, output_path: &str) -> Result<(), HuffmanError> {
    let data = std::fs::read(input_path)?;
    let text = decode_from_bytes(&data)?;
    write_new_file(output_path, text.as_bytes())
}

#[cfg(test)]
mod tests;

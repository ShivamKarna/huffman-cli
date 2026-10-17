// Encoded Header Format -> [char_count]-[tree_length]-[tree]-[packed_bits]
//
use crate::error::HuffmanError;
use crate::file_helpers::write_new_file;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Eq, PartialEq)]
pub struct Node {
    pub ch: Option<char>,
    pub freq: u32,
    pub left: Option<Box<Node>>,
    pub right: Option<Box<Node>>,
}

impl Node {
    fn leaf(char: Option<char>, freq: u32) -> Self {
        Self {
            ch: char,
            freq,
            left: None,
            right: None,
        }
    }

    fn internal(freq: u32, left: Node, right: Node) -> Self {
        Self {
            ch: None,
            freq,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

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

pub fn count_freq(text: &str) -> HashMap<char, usize> {
    let mut counts = HashMap::<char, usize>::new();
    for ch in text.chars() {
        *counts.entry(ch).or_insert(0) += 1;
    }
    counts
}

pub fn build_tree(counts: &HashMap<char, usize>) -> Option<Node> {
    if counts.is_empty() {
        return None;
    }

    let mut heap = BinaryHeap::new();

    for (&ch, &freq) in counts {
        heap.push(Node::leaf(Some(ch), freq as u32));
    }

    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();
        let freq = left.freq + right.freq;

        heap.push(Node::internal(freq, left, right));
    }

    heap.pop()
}

pub fn make_codes(node: &Node, prefix: String, codes: &mut HashMap<char, String>) {
    // a t
    if let Some(ch) = node.ch {
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

pub fn generate_code(root: &Node) -> HashMap<char, String> {
    let mut codes = HashMap::<char, String>::new();
    make_codes(root, String::new(), &mut codes);
    codes
}

// Encoding Part

pub fn serialize_tree(node: &Node, out_buffer: &mut Vec<u8>) {
    if let Some(ch) = node.ch {
        out_buffer.push(1); // 1 denoting leaf node
        out_buffer.extend_from_slice(&(ch as u32).to_le_bytes());
    } else {
        out_buffer.push(0); // 0 denoting internal node
        if let Some(left) = node.left.as_ref() {
            serialize_tree(left, out_buffer);
        }
        if let Some(right) = node.right.as_ref() {
            serialize_tree(right, out_buffer);
        }
    }
}

// turn the text into packed bytes, 8 bits per byte
pub fn pack_bits(content: &str, codes: HashMap<char, String>) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut current: u8 = 0;
    let mut bit_count: u8 = 0;

    for ch in content.chars() {
        for bit in codes[&ch].chars() {
            current = current << 1;
            if bit == '1' {
                current = current | 1;
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

// Decode and write to file.
pub fn encode_to_file(content: &str, output_path: &str) -> Result<(), HuffmanError> {
    let counts = count_freq(content);
    let root = build_tree(&counts).ok_or(HuffmanError::EmptyFile)?;
    let codes = generate_code(&root);

    let mut tree_bytes = Vec::new();
    serialize_tree(&root, &mut tree_bytes);

    let packed = pack_bits(content, codes);
    let total_chars = content.chars().count() as u64;

    let mut out_buffer = Vec::new();

    out_buffer.extend_from_slice(&total_chars.to_le_bytes());
    out_buffer.extend_from_slice(&(tree_bytes.len() as u32).to_le_bytes());
    out_buffer.extend_from_slice(&tree_bytes);
    out_buffer.extend_from_slice(&packed);

    write_new_file(output_path, &out_buffer)
}

// Decoding Part
pub fn deserialize_tree(data: &[u8], pos: &mut usize) -> Result<Node, HuffmanError> {
    if *pos >= data.len() {
        return Err(HuffmanError::CorruptedData);
    }
    let tag = data[*pos];
    *pos += 1;

    match tag {
        1 => {
            let bytes_slice = data
                .get(*pos..*pos + 4)
                .ok_or(HuffmanError::CorruptedData)?;

            let bytes = bytes_slice
                .try_into()
                .map_err(|_| HuffmanError::CorruptedData)?;

            *pos += 4;
            let num = u32::from_le_bytes(bytes); // first convert the let bytes to u32
            let ch = char::from_u32(num).ok_or(HuffmanError::CorruptedData)?; // then convert that u32 to the actual char
            Ok(Node::leaf(Some(ch), 0))
        }

        0 => {
            let left = deserialize_tree(data, pos)?;
            let right = deserialize_tree(data, pos)?;
            Ok(Node::internal(0, left, right))
        }
        _ => Err(HuffmanError::CorruptedData),
    }
}

// Walk the tree bit by bit: 0 = left, 1 = right, leaf = write the char and restart at the root
pub fn decode_bits(root: &Node, bytes: &[u8], total: u64) -> Result<String, HuffmanError> {
    let mut string_buf = String::new();

    // If root itself has 'ch' then, there might be only one distinct character in the file
    if let Some(ch) = root.ch {
        for _ in 0..total {
            // loop to get the total number of chars , that was provided by the header.
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

// Decode and write to file
pub fn decode_to_file(input_path: &str, output_path: &str) -> Result<(), HuffmanError> {
    let data = std::fs::read(input_path)?;

    if data.len() < 12 {
        return Err(HuffmanError::CorruptedData);
    }

    let mut total_bytes = [0u8; 8];
    total_bytes.copy_from_slice(&data[0..8]);
    let total = u64::from_le_bytes(total_bytes);

    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&data[8..12]);
    let tree_len = u32::from_le_bytes(len_bytes) as usize;

    let tree_end = 12 + tree_len;

    if tree_end > data.len() {
        return Err(HuffmanError::CorruptedData);
    }

    let mut pos = 12;
    let root = deserialize_tree(&data[..tree_end], &mut pos)?;

    let text = decode_bits(&root, &data[tree_end..], total)?;

    write_new_file(output_path, text.as_bytes())
}

#[cfg(test)]
mod tests;

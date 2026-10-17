use super::*;
use std::fs;

#[test]
fn count_characters() {
    let counts = count_freq("aab");
    assert_eq!(counts[&'a'], 2);
    assert_eq!(counts[&'b'], 1);
    assert_eq!(counts.len(), 2);
}

#[test]
fn empty_counts_give_no_tree() {
    let counts = HashMap::new();
    assert!(build_tree(&counts).is_none());
}

#[test]
fn root_freq_is_equal_to_sum() {
    let text = "abracadabra";
    let root = build_tree(&count_freq(text)).unwrap();
    assert_eq!(root.freq as usize, text.chars().count());
}

#[test]
fn single_char_gets_zero_code() {
    let root = build_tree(&count_freq("aaaa")).unwrap();
    let codes = generate_code(&root);
    assert_eq!(codes[&'a'], "0");
}

#[test]
fn codes_are_prefix_free() {
    let root = build_tree(&count_freq("abracadabra")).unwrap();
    let codes = generate_code(&root);
    assert_eq!(codes.len(), 5);

    for (c1, code1) in &codes {
        for (c2, code2) in &codes {
            if c1 != c2 {
                assert!(
                    !code2.starts_with(code1.as_str()),
                    "{} is a prefix of {}",
                    code1,
                    code2
                )
            }
        }
    }
}

#[test]
fn frequent_chars_get_shorter_code() {
    let root = build_tree(&count_freq("abracadabra")).unwrap();
    let codes = generate_code(&root);
    // a's freq is 5, c and d is only 1.
    assert!(codes[&'a'].len() <= codes[&'c'].len());
    assert!(codes[&'a'].len() <= codes[&'d'].len());
}

#[test]
fn serialize_leaf_node() {
    let leaf = Node::leaf(Some('a'), 1);
    let mut buf = Vec::new();
    serialize_tree(&leaf, &mut buf);
    assert_eq!(buf, vec![1, 97, 0, 0, 0]);
}

#[test]
fn serialize_internal_node() {
    let tree = Node::internal(2, Node::leaf(Some('a'), 1), Node::leaf(Some('b'), 1));
    let mut buf = Vec::new();
    serialize_tree(&tree, &mut buf);
    assert_eq!(buf, vec![0, 1, 97, 0, 0, 0, 1, 98, 0, 0, 0])
}
#[test]
fn pack_bits_pads_last_byte() {
    let mut codes = HashMap::new();
    codes.insert('a', "101".to_string());
    codes.insert('b', "11".to_string());
    assert_eq!(pack_bits("ab", codes), vec![0b1011_1000]);
}

#[test]
fn pack_bits_exact_byte() {
    let mut codes = HashMap::new();
    codes.insert('a', "1".to_string());
    codes.insert('b', "0".to_string());
    assert_eq!(pack_bits("aaaaaaaa", codes), vec![0xFF]);
}

#[test]
fn deserialize_tree_test() {
    let data = [1, 97, 0, 0, 0];
    let mut pos = 0;
    let node = deserialize_tree(&data, &mut pos).unwrap();
    assert_eq!(node, Node::leaf(Some('a'), 0));
    assert_eq!(pos, 5);
}

#[test]
fn decode_bits_test() {
    let data = [0, 1, 97, 0, 0, 0, 1, 98, 0, 0, 0];
    let mut pos = 0;
    let node = deserialize_tree(&data, &mut pos).unwrap();

    let packed = [0b0100_0000];
    let decoded = decode_bits(&node, &packed, 2).unwrap();
    assert_eq!(decoded, "ab");
}
#[test]
fn decode_bits_abracadabra() {
    let tree_bytes = [
        // it's like : [ 0x6E, 0x8A, 0xDC ]
        0, 1, 97, 0, 0, 0, // a
        0, // Z
        0, // X
        1, 99, 0, 0, 0, // c
        1, 100, 0, 0, 0, // d
        0, // Y
        1, 98, 0, 0, 0, // b
        1, 114, 0, 0, 0, // r
    ];

    let mut pos = 0;

    let root = deserialize_tree(&tree_bytes, &mut pos).unwrap();
    assert_eq!(pos, 29);

    let packed = [0x6E, 0x8A, 0xDC];

    let decoded = decode_bits(&root, &packed, 11).unwrap();
    assert_eq!(decoded, "abracadabra");
}

#[test]
fn encode_writes_correct_layout() {
    let path = std::env::temp_dir().join("huff_test_layout.en.huff");
    let path_str = path.to_str().unwrap();

    let _ = fs::remove_file(&path); // clean up from an earlier failed run

    encode_to_file("abracadabra", path_str).unwrap();

    let data = fs::read(&path).unwrap();

    assert_eq!(u64::from_le_bytes(data[0..8].try_into().unwrap()), 11);

    let tree_len = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    assert!(data.len() > 12 + tree_len);

    assert_eq!(tree_len, 29);

    fs::remove_file(&path).unwrap();
}

#[test]
fn encode_refuses_to_overwrite() {
    let path = std::env::temp_dir().join("huff_test_overwrite.en.huff");
    let path_str = path.to_str().unwrap();
    let _ = fs::remove_file(&path);

    encode_to_file("hello", path_str).unwrap();
    let second = encode_to_file("hello", path_str);
    assert!(matches!(second, Err(HuffmanError::OutputExists(_))));

    fs::remove_file(&path).unwrap();
}

#[test]
fn encode_empty_text_is_error() {
    let path = std::env::temp_dir().join("huff_test.en.huff");
    let result = encode_to_file("", path.to_str().unwrap());
    assert!(matches!(result, Err(HuffmanError::EmptyFile)));
}

fn round_trip(text: &str) -> String {
    let counts = count_freq(text);
    let root = build_tree(&counts).unwrap();
    let codes = generate_code(&root);

    let mut tree_bytes = Vec::new();
    serialize_tree(&root, &mut tree_bytes);
    let packed = pack_bits(text, codes);

    let mut pos = 0;
    let new_root = deserialize_tree(&tree_bytes, &mut pos).unwrap();
    decode_bits(&new_root, &packed, text.chars().count() as u64).unwrap()
}

#[test]
fn everything_works() {
    assert_eq!(round_trip("hello world\n"), "hello world\n");
    assert_eq!(round_trip("aaaa"), "aaaa");
    assert_eq!(round_trip("नमस्ते 😀"), "नमस्ते 😀");
}

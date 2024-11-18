use dmmt_jpeg_encoder::binary_stream::BitWriter;
use dmmt_jpeg_encoder::huffman::{CodingError, HuffmanCoder, HuffmanTree};
use std::io::Write;

fn main() -> Result<(), CodingError> {
    // symbol-frequency pairs
    let syms_and_freqs = [(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12)];

    let mut tree = HuffmanTree::new(&syms_and_freqs);
    println!("huffman tree\n{}", tree);
    tree.reorder_right_growing();
    println!("right-growing huffman\n{}", tree);
    tree.replace_onestar();
    println!("right-growing huffman without 1*\n{}", tree);

    let sequence_to_encode = [3, 3, 3, 2, 1, 4, 5, 3, 3, 3];

    let coder = HuffmanCoder::new(&tree);
    let mut encoded_buffer: Vec<u8> = Vec::new();
    let mut writer = BitWriter::new(&mut encoded_buffer, true);
    coder.encode_sequence(&sequence_to_encode, &mut writer)?;
    let _ = writer.flush();
    println!("sequence to encode\n{:?}", sequence_to_encode);
    println!("encoded sequence\n{:?}", encoded_buffer);

    let mut decoded_buffer: Vec<u32> = Vec::new();
    coder.decode_sequence(encoded_buffer.as_slice(), &mut decoded_buffer)?;
    println!("decoded sequence\n{:?}", decoded_buffer);
    Ok(())
}

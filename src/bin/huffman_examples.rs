use dmmt_jpeg_encoder::binary_stream::BitWriter;
use dmmt_jpeg_encoder::huffman::{CodingError, HuffmanCoder, HuffmanTree};
use std::io::Write;

fn main() -> Result<(), CodingError> {
    // symbol-frequency pairs
    let syms_and_freqs = vec![(0, 10), (1, 2), (2, 24), (3, 340), (4, 10), (5, 11)];

    let mut tree = HuffmanTree::new(&syms_and_freqs);
    println!("huffman tree\n{}", tree);
    tree.correct_ordering();
    println!("right-growing huffman\n{}", tree);
    tree.replace_onestar();
    println!("right-growing huffman without 1*\n{}", tree);

    let sequence_to_encode = vec![3, 3, 3, 2, 1, 4, 5, 3, 3, 3];

    let coder = HuffmanCoder::new(&tree);
    let mut my_output: Vec<u8> = vec![];
    let mut writer = BitWriter::new(&mut my_output, true);
    coder.encode_sequence(&sequence_to_encode, &mut writer)?;
    let _ = writer.flush();
    println!("encoding sequence\n{:?}", sequence_to_encode);
    println!("encoded_sequence\n{:?}", my_output);

    let mut my_decoded: Vec<u32> = vec![];
    coder.decode_sequence(my_output.as_slice(), &mut my_decoded)?;
    println!("decoded sequence\n{:?}", my_decoded);
    Ok(())
}

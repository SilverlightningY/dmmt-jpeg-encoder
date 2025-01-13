use std::io::Write;

use dmmt_jpeg_encoder::binary_stream::BitWriter;
use dmmt_jpeg_encoder::huffman::code::HuffmanCodeGenerator;
use dmmt_jpeg_encoder::huffman::encoder::{HuffmanTranslator, HuffmanWriter};
use dmmt_jpeg_encoder::huffman::length_limited::LengthLimitedHuffmanCodeGenerator;
use dmmt_jpeg_encoder::huffman::tree::HuffmanTree;
use dmmt_jpeg_encoder::huffman::SymbolFrequency;

fn main() {
    let syms_and_freqs = [
        (0, 13),
        (1, 14),
        (2, 25),
        (3, 26),
        (4, 28),
        (5, 60),
        (6, 120),
    ];

    let mut generator = LengthLimitedHuffmanCodeGenerator::new(4);
    let mut tree = HuffmanTree::new(&syms_and_freqs, &mut generator);
    tree.replace_onestar();

    let syms_and_freqs = syms_and_freqs.map(SymbolFrequency::from);

    let mut output: Vec<u8> = Vec::new();
    let mut writer = BitWriter::new(&mut output, true);
    let mut code_lengths = generator.generate_with_symbols(&syms_and_freqs);
    code_lengths[0].length += 1;
    let translator = HuffmanTranslator::from(&code_lengths);
    let mut encoder = HuffmanWriter::new(&translator, &mut writer);

    /* an example sequence to encode that roughly matches the relative frequencies at the beginning */
    let encoding_sequence: Vec<u8> = vec![
        0, 6, 4, 4, 3, 3, 6, 5, 6, 2, 6, 1, 6, 5, 3, 5, 6, 6, 2, 2, 6, 5, 6, 5, 4, 1,
    ];
    let _ = encoder.write_all(&encoding_sequence);
    let _ = encoder.flush();

    /* have the tree decode the sequence */
    let mut decoded: Vec<u8> = Vec::new();
    let _ = tree.decode_sequence(&mut output.as_slice(), &mut decoded);

    println!("encoded sequence \n {:?}", output);
    println!("original sequence \n {:?}", encoding_sequence);
    println!("decoded sequence \n {:?}", decoded);
}

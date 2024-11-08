use dmmt_jpeg_encoder::binary_stream::BitWriter;

fn main() {
    let mut my_output: Vec<u8> = vec![];
    let mut writer = BitWriter::new(&mut my_output);

    // 10 bit pattern: 1110001100 (write 1 mil times)
    for _i in 0..1000000 {
        // write 1 bit at a time
        writer.write_bits(&[0xFF], 1).expect("write failed");
        writer.write_bits(&[0xFF], 1).expect("write failed");
        writer.write_bits(&[0xFF], 1).expect("write failed");
        writer.write_bits(&[0x00], 1).expect("write failed");
        writer.write_bits(&[0x00], 1).expect("write failed");
        writer.write_bits(&[0x00], 1).expect("write failed");
        writer.write_bits(&[0xFF], 1).expect("write failed");
        writer.write_bits(&[0xFF], 1).expect("write failed");
        writer.write_bits(&[0x00], 1).expect("write failed");
        writer.write_bits(&[0x00], 1).expect("write failed");
    }
    // 10 bit pattern results in repeating 5 byte pattern
    let expected_pattern: Vec<u8> =
        vec![0b11100011, 0b00111000, 0b11001110, 0b00110011, 0b10001100];
    for (byte_index, byte) in my_output.iter().enumerate() {
        let exp_byte = expected_pattern[byte_index % 5];
        for bit_index in 0..8 {
            if ((*byte) & 1_u8.rotate_left(bit_index)) != (exp_byte & 1_u8.rotate_left(bit_index)) {
                println!(
                    "bit mismatch at position {} in byte {}",
                    bit_index, byte_index
                )
            }
        }
    }
    println!("bit write and read finished")
}

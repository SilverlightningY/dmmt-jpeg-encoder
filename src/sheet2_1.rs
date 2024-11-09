mod binary_stream;
use binary_stream::BitWriter;

fn main() {
    let mut my_output: Vec<u8> = vec![];
    let mut writer = BitWriter::new(&mut my_output, false);

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
    let mut on_item = 0;
    for byte in &my_output {
        let exp_byte = expected_pattern[on_item % 5];
        for bit_index in 0..8 {
            if ((*byte) & 1_u8.rotate_left(bit_index)) != (exp_byte & 1_u8.rotate_left(bit_index)) {
                println!("bit mismatch at position {} in byte {}", bit_index, on_item)
            }
        }
        on_item += 1;
    }
    println!("bit write and read finished")
}

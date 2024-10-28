use std::io;
use std::io::Write;

/// State for writing individual bits to a Writer
pub struct BitWriter<'a, T: Write> {
    /// the underlying output stream
    writer: &'a mut T,
    /// buffer of individual bits not yet written
    buffer: u8,
    /// how many bits are waiting to be written
    buffer_space_used: u8,
}

impl<'a, T: Write> BitWriter<'a, T> {
    fn new(writer: &'a mut T) -> BitWriter<'a, T> {
        BitWriter {
            writer,
            buffer: 0,
            buffer_space_used: 0,
        }
    }

    /// write a non-byte-aligned number of bits
    ///
    /// buf: a byte array containing a contigous block
    /// count: how many bits of buf to write
    ///
    /// returns the number of byte writes incurred onto
    /// the underlying stream, but does not guarantee that
    /// all bits have been written, use flush to write
    /// any remaining bits.
    fn write_bits(&mut self, buf: &[u8], count: usize) -> Result<usize, io::Error> {
        let mut remaining_bits_offset = 0;
        let mut bytes_written = 0;
        if self.buffer_space_used == 0 {
            // this is efficient for large blocks of byte writes
            let quick_byte_count = count / 8;
            bytes_written = self.writer.write(&buf[0..quick_byte_count])?;
            remaining_bits_offset = quick_byte_count * 8;
        }
        for bit_index in remaining_bits_offset..count {
            // this isn't (for large blocks of bites)
            let byte_index = bit_index / 8;
            let bit_index = bit_index % 8;
            let bit_val: bool = (buf[byte_index] & 0b10000000_u8.rotate_right(bit_index as u32)) > 0;
            if bit_val {
                self.buffer |= 0b10000000_u8.rotate_right(self.buffer_space_used as u32);
            } else {
                self.buffer &= 0b01111111_u8.rotate_right(self.buffer_space_used as u32);
            }
            self.buffer_space_used += 1;
            if self.buffer_space_used == 8 {
                bytes_written += self.writer.write(&[self.buffer])?;
                self.buffer_space_used = 0;
                self.buffer = 0; // depended upon in flush()
            }
        }
        Ok(bytes_written)
    }
}

impl<'a, T: Write> Write for BitWriter<'a, T> {
    /// Writing of byte arrays into the bit writer (for performance)
    ///
    /// Warning: Even when the returned number in the result equals
    ///          the length of the input buffer, not all bits of the
    ///          input may have been written (because of possible
    ///          single bits in BitWriters buffer)
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.write_bits(buf, buf.len() * 8)
    }

    /// Flush all bits and the underlying writer;
    ///
    /// If there are non-byte-aligned bits still
    /// in the buffer, they will be written to the output
    /// with 0 padding to the next byte;
    fn flush(&mut self) -> Result<(), io::Error> {
        if self.buffer_space_used != 0 {
            self.writer.write_all(&[self.buffer])?;
            self.buffer = 0;
            self.buffer_space_used = 0;
        }
        self.writer.flush()
    }
}

#[cfg(test)]
mod test {
    use super::BitWriter;
    use std::io::Write;

    #[test]
    fn byte_mode_test() {
        let mut my_output: Vec<u8> = vec![];
        let mut writer = BitWriter::new(&mut my_output);
        let input: &[u8] = &[72, 65, 76, 76, 79];
        writer.write(input).expect("should not fail");
        writer.flush().expect("flushing should not fail");
        assert_eq!(my_output[0], 72);
        assert_eq!(my_output[1], 65);
        assert_eq!(my_output[2], 76);
        assert_eq!(my_output[3], 76);
        assert_eq!(my_output[4], 79);
        assert_eq!(my_output.len(), 5);
    }

    #[test]
    fn bit_mode_test() {
        let mut my_output: Vec<u8> = vec![];
        let mut writer = BitWriter::new(&mut my_output);
        // write 0x11000011 0x11110000 (in MSb notation)
        writer.write_bits(&[0xFF], 2).expect("ERR");
        writer.write_bits(&[0x00], 4).expect("ERR");
        writer.write_bits(&[0xFF], 2).expect("ERR");
        writer.write_bits(&[0xFF], 4).expect("ERR");
        writer.flush().expect("ERR");
        assert_eq!(my_output.len(), 2);
        assert_eq!(my_output[0], 195);
        assert_eq!(my_output[1], 15 << 4);
    }

    #[test]
    fn mixed_mode_test() {
        let mut my_output: Vec<u8> = vec![];
        let mut writer = BitWriter::new(&mut my_output);
	// 0b111
        writer.write_bits(&[0xFF], 3).expect("ERR");
	// 0b11100000 00100000 01010000 100
        writer.write(&[1, 2, 4 | 128]).expect("ERR");
        writer.flush().expect("ERR");
        assert_eq!(my_output.len(), 4);
        assert_eq!(my_output[0], 224);
        assert_eq!(my_output[1], 32);
        assert_eq!(my_output[2], 80);
        assert_eq!(my_output[3], 128);
    }
}

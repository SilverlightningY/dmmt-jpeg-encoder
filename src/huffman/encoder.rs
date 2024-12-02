use crate::binary_stream::BitWriter;
use std::io::Write;

use super::{code::HuffmanCodeGenerator, length_limited::LengthLimitedHuffmanCodeGenerator};

type CodeWord = u16;
type BitLength = u8;
pub struct HuffmanEncoder<'a, T: Write> {
    writer: &'a mut BitWriter<'a, T>,
    pub symbols_to_code_words: [(CodeWord, BitLength); 256],
    symbol_exists: [bool; 256],
}

pub type EncoderInputSymbol = u8;
pub type EncodedLength = usize;

impl<'a, T: Write> HuffmanEncoder<'a, T> {
    pub fn new(
        writer: &'a mut BitWriter<'a, T>,
        symbols: &[(EncoderInputSymbol, EncodedLength)],
    ) -> Self {
        if symbols.len() > 256 {
            panic!("can't encode more than 256 different symbols");
        }

        if !symbols.iter().rev().is_sorted_by_key(|s| s.1) {
            panic!("symbols-array needs to be sorted by descending code word length");
        }

        if symbols.is_empty() {
            panic!("the set of input symbols is empty")
        }

        if symbols[0].1 >= 16 {
            panic!("maximum code word length allowed in input is 15 bits");
        }

        let mut max_len = 0;
        // remove the 1* pattern by having the longest pattern occur once
        let symbols: Vec<(EncoderInputSymbol, EncodedLength)> = symbols
            .iter()
            .enumerate()
            .map(|(idx, &s)| -> (EncoderInputSymbol, EncodedLength) {
                if idx == 0 {
                    max_len = s.1 + 1;
                    return (s.0, max_len);
                }
                s
            })
            .collect();

        let mut encoder = HuffmanEncoder {
            writer,
            symbols_to_code_words: [(0, 0); 256],
            symbol_exists: [false; 256],
        };

        symbols.iter().for_each(|&(sym, len)| {
            if encoder.symbol_exists[sym as usize] {
                panic!(
                    "symbol {} is encountered for the second time in the set of input symbols",
                    sym
                );
            }
            encoder.symbol_exists[sym as usize] = true;
            encoder.symbols_to_code_words[sym as usize] = (0, len as u8);
        });

        // code word calculation
        symbols
            .iter()
            .rev()
            .skip(1)
            .zip(symbols.iter().rev())
            .for_each(|(&(sym, _), &(previous_sym, previous_len))| {
                let previous_code = encoder.symbols_to_code_words[previous_sym as usize].0;
                let increment = 1 << (max_len - previous_len);
                let current_code = previous_code + increment;
                encoder.symbols_to_code_words[sym as usize].0 = current_code;
            });

        // left-justify code words in 16 bit integer
        for i in 0..256 {
            encoder.symbols_to_code_words[i as usize].0 <<= 16 - max_len;
        }

        encoder
    }

    pub fn from_symbols_and_frequencies(
        symbols_and_frequencies: &[(u8, usize)],
        limit: usize,
        writer: &'a mut BitWriter<'a, T>,
    ) -> Self {
        if !symbols_and_frequencies.iter().is_sorted_by_key(|x| x.1) {
            panic!("symbols need to be in order of frequency");
        }

        let freqs: Vec<usize> = symbols_and_frequencies.iter().map(|&(_, f)| f).collect();
        let syms: Vec<u8> = symbols_and_frequencies.iter().map(|&(s, _)| s).collect();

        let mut generator = LengthLimitedHuffmanCodeGenerator::new(limit);
        let code = generator.generate(&freqs);

        let combined_code: Vec<(u8, usize)> = syms
            .iter()
            .zip(code.iter())
            .map(|(&a, &b)| (a, b))
            .collect();
        HuffmanEncoder::new(writer, &combined_code)
    }
}

impl<'a, T: Write> Write for HuffmanEncoder<'a, T> {
    /* "Ich kenne meine Daten"-Version */
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &s in buf {
            let code = self.symbols_to_code_words[s as usize];
            let bytes = code.0.to_be_bytes();
            self.writer.write_bits(&bytes, code.1.into())?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod test {
    use super::HuffmanEncoder;
    use crate::{
        binary_stream::BitWriter, huffman::code::HuffmanCodeGenerator,
        huffman::length_limited::LengthLimitedHuffmanCodeGenerator,
    };
    use std::io::Write;

    #[test]
    #[should_panic]
    fn test_unsorted_symbols() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = BitWriter::new(&mut output, true);
        let unsorted_symbols = vec![(0, 1), (1, 5), (2, 4), (3, 3)];
        let _ = HuffmanEncoder::new(&mut writer, &unsorted_symbols);
    }

    #[test]
    #[should_panic]
    fn test_max_code_length_too_long() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = BitWriter::new(&mut output, true);
        let symbols = vec![(0, 16), (1, 5), (2, 4), (3, 3)];
        let _ = HuffmanEncoder::new(&mut writer, &symbols);
    }

    const TEST_SYMBOL_SEQUENCE: &[u8] = &[1, 3, 2, 2, 7, 5, 4, 4, 1];
    const TEST_BYTE_SEQUENCE: &[u8] = &[0b01110111, 0b10111101, 0b00001110, 0b11100100];
    const SYMBOLS_AND_FREQUENCIES_ODD_LEN: &[(u8, usize); 7] =
        &[(1, 17), (2, 3), (3, 12), (4, 3), (5, 18), (6, 12), (7, 13)];

    #[test]
    fn test_coder_encode() -> std::io::Result<()> {
        let mut sorted_syms: Vec<(u8, usize)> = SYMBOLS_AND_FREQUENCIES_ODD_LEN.to_vec();
        sorted_syms.sort_by_key(|x| x.1);

        let mut output: Vec<u8> = Vec::new();
        let mut writer = BitWriter::new(&mut output, false);
        let mut encoder =
            HuffmanEncoder::from_symbols_and_frequencies(&sorted_syms, 10, &mut writer);

        encoder.write_all(TEST_SYMBOL_SEQUENCE)?;
        encoder.flush()?;

        assert_eq!(
            TEST_BYTE_SEQUENCE.len(),
            output.iter().len(),
            "decoded sequence length different from expect"
        );

        for (index, &byte) in output.iter().enumerate() {
            assert_eq!(
                byte, TEST_BYTE_SEQUENCE[index],
                "Byte at index {} does not match",
                index
            );
        }

        Ok(())
    }
}

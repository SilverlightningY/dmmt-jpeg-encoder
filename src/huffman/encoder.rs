use crate::binary_stream::BitWriter;
use std::io::{self, Write};

use super::{Symbol, SymbolCodeLength};

type CodeBitPattern = u16;

struct CodeWord {
    bit_pattern: CodeBitPattern,
    length: usize,
}

impl From<(CodeBitPattern, usize)> for CodeWord {
    fn from(value: (CodeBitPattern, usize)) -> Self {
        Self {
            bit_pattern: value.0,
            length: value.1,
        }
    }
}

pub struct HuffmanEncoder<'a, T: Write> {
    writer: &'a mut BitWriter<'a, T>,
    code_word_lookup_table: [Option<CodeWord>; Symbol::MAX as usize],
}

impl<'a, T: Write> HuffmanEncoder<'a, T> {
    pub fn new(writer: &'a mut BitWriter<'a, T>, code_lengths: &[SymbolCodeLength]) -> Self {
        Self::validate_input_code_lengths(code_lengths);
        let mut encoder = HuffmanEncoder {
            writer,
            code_word_lookup_table: [const { None }; Symbol::MAX as usize],
        };
        encoder.fill_lookup_table(code_lengths);
        encoder
    }

    fn fill_lookup_table(&mut self, code_lengths: &[SymbolCodeLength]) {
        self.insert_initial_code_word(code_lengths);
        self.insert_following_code_words(code_lengths);
    }

    fn insert_initial_code_word(&mut self, code_lengths: &[SymbolCodeLength]) {
        let last_code_length = code_lengths.last().expect("code_lengths must not be empty");
        let code_word = Self::create_initial_code_word(last_code_length);
        self.set_code_word_for_symbol(last_code_length.symbol, code_word);
    }

    fn insert_following_code_words(&mut self, code_lengths: &[SymbolCodeLength]) {
        for (current, previous) in code_lengths
            .iter()
            .rev()
            .skip(1)
            .zip(code_lengths.iter().rev())
        {
            self.ensure_symbol_was_not_inserted_before(current.symbol);
            let code_word = self.create_code_word(current.length, previous.symbol);
            self.set_code_word_for_symbol(current.symbol, code_word);
        }
    }

    fn create_initial_code_word(code_length: &SymbolCodeLength) -> CodeWord {
        CodeWord {
            bit_pattern: 0,
            length: code_length.length,
        }
    }

    fn create_code_word(&self, length: usize, previous_symbol: Symbol) -> CodeWord {
        let previous_code_word = self
            .get_code_word_for_symbol(previous_symbol)
            .as_ref()
            .expect("Symbol not in lookup table");
        let bit_pattern = Self::calculate_bit_pattern(previous_code_word);
        CodeWord {
            length,
            bit_pattern,
        }
    }

    fn calculate_bit_pattern(previous_code_word: &CodeWord) -> CodeBitPattern {
        let increment = 1 << (CodeBitPattern::BITS - previous_code_word.length as u32);
        previous_code_word.bit_pattern + increment
    }

    fn set_code_word_for_symbol(&mut self, symbol: Symbol, code_word: CodeWord) {
        self.code_word_lookup_table[symbol as usize] = Some(code_word);
    }

    fn get_code_word_for_symbol(&self, symbol: Symbol) -> &Option<CodeWord> {
        &self.code_word_lookup_table[symbol as usize]
    }

    fn ensure_symbol_was_not_inserted_before(&self, symbol: Symbol) {
        if self.symbol_exists(symbol) {
            panic!(
                "symbol {} is encountered for the second time in the set of input symbols",
                symbol
            );
        }
    }

    fn symbol_exists(&self, symbol: Symbol) -> bool {
        self.code_word_lookup_table[symbol as usize].is_some()
    }

    fn validate_input_code_lengths(code_lengths: &[SymbolCodeLength]) {
        if code_lengths.is_empty() {
            panic!("the set of input symbols must not be empty");
        }

        if code_lengths.len() > Symbol::MAX as usize {
            panic!("can't encode more than {} different symbols", Symbol::MAX);
        }

        if !code_lengths.iter().rev().is_sorted_by_key(|s| s.length) {
            panic!("symbols-array needs to be sorted by descending code word length");
        }

        if code_lengths[0].length as u32 > CodeBitPattern::BITS {
            panic!(
                "maximum code word length allowed in input is {} bits",
                CodeBitPattern::BITS
            );
        }
    }
}

impl<'a, T: Write> Write for HuffmanEncoder<'a, T> {
    fn write(&mut self, buf: &[Symbol]) -> io::Result<usize> {
        for &symbol in buf {
            let code = self
                .get_code_word_for_symbol(symbol)
                .as_ref()
                .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;
            let bytes = code.bit_pattern.to_be_bytes();
            self.writer.write_bits(&bytes, code.length)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod test {
    use std::io::{self, Write};

    use super::super::{
        code::HuffmanCodeGenerator, length_limited::LengthLimitedHuffmanCodeGenerator,
        SymbolCodeLength, SymbolFrequency,
    };
    use super::{CodeWord, HuffmanEncoder};
    use crate::binary_stream::BitWriter;

    #[test]
    #[should_panic]
    fn test_unsorted_symbols() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = BitWriter::new(&mut output, true);
        let unsorted_symbols = [(0, 1), (1, 5), (2, 4), (3, 3)].map(SymbolCodeLength::from);
        let _ = HuffmanEncoder::new(&mut writer, &unsorted_symbols);
    }

    #[test]
    #[should_panic]
    fn test_max_code_length_too_long() {
        let mut output: Vec<u8> = Vec::new();
        let mut writer = BitWriter::new(&mut output, true);
        let symbols = [(0, 17), (1, 5), (2, 4), (3, 3)].map(SymbolCodeLength::from);
        let _ = HuffmanEncoder::new(&mut writer, &symbols);
    }

    const TEST_SYMBOL_SEQUENCE: &[u8] = &[
        27, 17, 7, 31, 22, 12, 2, 29, 21, 19, 11, 9, 1, 30, 25, 15, 5, 24, 14, 4, 20, 10, 26, 23,
        16, 13, 6, 3, 32, 28, 18, 8,
    ];
    const TEST_BYTE_SEQUENCE: &[u8] = &[
        0b00000100, 0b01101000, 0b10101100, 0b11110000, 0b10001100, 0b10100111, 0b01001010,
        0b11011010, 0b11101011, 0b11110000, 0b11000111, 0b00101100, 0b11110100, 0b11010111,
        0b01101101, 0b11111000, 0b11100111, 0b10101110, 0b11111100, 0b11110111, 0b11101111,
        0b11000000,
    ];

    #[rustfmt::skip]
    const SYMBOLS_AND_FREQUENCIES_ODD_LEN: &[(u8, usize); 32] = &[
        (1, 14), (2, 30), (3, 4), (4, 7), (5, 9), (6, 4), (7, 42), (8, 1), (9, 14), (10, 5), 
        (11, 14), (12, 30), (13, 4), (14, 7), (15, 9), (16, 4), (17, 42), (18, 1), (19, 14), 
        (20,5), (21, 14), (22, 30), (23, 4), (24, 7), (25, 9), (26, 4), (27, 42), (28, 1), 
        (29, 14), (30, 12), (31, 32), (32, 1)];

    fn create_test_encoder<'a, T: Write>(
        sorted_frequencies: &[SymbolFrequency],
        length: usize,
        writer: &'a mut BitWriter<'a, T>,
    ) -> HuffmanEncoder<'a, T> {
        let mut generator = LengthLimitedHuffmanCodeGenerator::new(length);
        let mut code_lengths = generator.generate_with_symbols(sorted_frequencies);
        code_lengths[0].length += 1;
        HuffmanEncoder::new(writer, &code_lengths)
    }

    #[test]
    fn test_coder_encode() -> io::Result<()> {
        let mut sorted_syms = SYMBOLS_AND_FREQUENCIES_ODD_LEN.map(SymbolFrequency::from);
        sorted_syms.sort_by_key(|x| x.frequency);

        let mut output: Vec<u8> = Vec::new();
        let mut writer = BitWriter::new(&mut output, false);
        let mut encoder = create_test_encoder(&sorted_syms, 6, &mut writer);

        encoder.write_all(TEST_SYMBOL_SEQUENCE)?;
        encoder.flush()?;

        assert_eq!(
            TEST_BYTE_SEQUENCE.len(),
            output.len(),
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

    #[test]
    fn test_calculate_bit_pattern_one() {
        let previous_code_word = CodeWord {
            bit_pattern: 0b1100_0000_0000_0000,
            length: 4,
        };
        let pattern = HuffmanEncoder::<Vec<u8>>::calculate_bit_pattern(&previous_code_word);
        let expected_pattern = 0b1101_0000_0000_0000u16;
        assert_eq!(pattern, expected_pattern, "Pattern does not match");
    }

    #[test]
    fn test_calculate_bit_pattern_two() {
        let previous_code_word = CodeWord {
            bit_pattern: 0b1101_0000_0000_0000u16,
            length: 5,
        };
        let pattern = HuffmanEncoder::<Vec<u8>>::calculate_bit_pattern(&previous_code_word);
        let expected_pattern = 0b1101_1000_0000_0000u16;
        assert_eq!(pattern, expected_pattern, "Pattern does not match");
    }

    #[test]
    fn test_calculate_bit_pattern_three() {
        let previous_code_word = CodeWord {
            bit_pattern: 0b1111_0000_0000_0000u16,
            length: 5,
        };
        let pattern = HuffmanEncoder::<Vec<u8>>::calculate_bit_pattern(&previous_code_word);
        let expected_pattern = 0b1111_1000_0000_0000u16;
        assert_eq!(pattern, expected_pattern, "Pattern does not match");
    }
}
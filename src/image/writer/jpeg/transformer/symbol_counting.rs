use crate::huffman::{
    code::HuffmanCodeGenerator, length_limited::LengthLimitedHuffmanCodeGenerator,
    SymbolCodeLength, SymbolFrequency,
};

use super::categorize::CategorizedBlock;

macro_rules! counter {
    ($name:ident; $size:literal) => {
        struct $name {
            symbol_frequencies: [usize; $size],
        }

        impl $name {
            fn new() -> Self {
                Self {
                    symbol_frequencies: [usize::default(); $size],
                }
            }

            fn increment_symbol(&mut self, symbol: u8) {
                self.symbol_frequencies[symbol as usize] += 1;
            }

            fn to_symbol_frequencies(&self) -> Vec<SymbolFrequency> {
                (0..=u8::MAX)
                    .into_iter()
                    .zip(self.symbol_frequencies.iter().copied())
                    .filter(|&(_, f)| f > 0)
                    .map(SymbolFrequency::from)
                    .collect()
            }
        }
    };
}

counter!(DCCounter; 16);
counter!(ACCounter; 256);

pub struct HuffmanCount {
    ac_count: Vec<SymbolFrequency>,
    dc_count: Vec<SymbolFrequency>,
}

impl HuffmanCount {
    pub fn generate_ac_huffman_code(&self) -> Vec<SymbolCodeLength> {
        generate_code_lengths(&self.ac_count)
    }

    pub fn generate_dc_huffman_code(&self) -> Vec<SymbolCodeLength> {
        generate_code_lengths(&self.dc_count)
    }
}

impl<'a> FromIterator<&'a CategorizedBlock> for HuffmanCount {
    fn from_iter<T: IntoIterator<Item = &'a CategorizedBlock>>(blocks: T) -> Self {
        let mut dc_counter = DCCounter::new();
        let mut ac_counter = ACCounter::new();

        for block in blocks {
            dc_counter.increment_symbol(block.dc_symbol());
            for ac_symbol in block.iter_ac_symbols() {
                ac_counter.increment_symbol(ac_symbol);
            }
        }

        let mut ac_count = ac_counter.to_symbol_frequencies();
        sort_by_frequency(&mut ac_count);
        let mut dc_count = dc_counter.to_symbol_frequencies();
        sort_by_frequency(&mut dc_count);

        Self { ac_count, dc_count }
    }
}

impl<'a, T> From<T> for HuffmanCount
where
    T: IntoIterator<Item = &'a CategorizedBlock>,
{
    fn from(value: T) -> Self {
        Self::from_iter(value)
    }
}

fn generate_code_lengths(symfreqs: &[SymbolFrequency]) -> Vec<SymbolCodeLength> {
    let mut generator = LengthLimitedHuffmanCodeGenerator::new(15);
    let mut symlens = generator.generate_with_symbols(symfreqs);
    symlens[0].length += 1;
    symlens
}

fn sort_by_frequency(symbol_frequencies: &mut [SymbolFrequency]) {
    symbol_frequencies.sort_by_key(|s| s.frequency);
}

#[cfg(test)]
mod test {
    use crate::huffman::SymbolFrequency;

    use super::{
        super::super::transformer::{
            categorize::{CategoryEncodedInteger, LeadingZerosToken},
            CategorizedBlock,
        },
        HuffmanCount,
    };

    #[test]
    fn test_count_symbols() {
        let test_blocks_channel_1: Vec<CategorizedBlock> = vec![
            CategorizedBlock::new(
                CategoryEncodedInteger::from(30), // DC symbol: 5
                vec![
                    LeadingZerosToken::new(0, 300), // AC symbol: 0b00001001 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(4, 5),   // AC symbol: 0b01000011 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
            CategorizedBlock::new(
                CategoryEncodedInteger::from(0), // DC symbol: 0
                vec![
                    LeadingZerosToken::new(0, 600), // AC symbol: 0b00001010 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(4, 15),  // AC symbol: 0b01000100 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
        ];
        let test_blocks_channel_2: Vec<CategorizedBlock> = vec![
            CategorizedBlock::new(
                CategoryEncodedInteger::from(60), // DC symbol: 6
                vec![
                    LeadingZerosToken::new(0, 100), // AC symbol: 0b00000111 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(2, 7),   // AC symbol: 0b00100011 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
            CategorizedBlock::new(
                CategoryEncodedInteger::from(1), // DC symbol: 1
                vec![
                    LeadingZerosToken::new(0, 900), // AC symbol: 0b00001010 x
                    LeadingZerosToken::new(15, 0),  // AC symbol: 0b11110000 x
                    LeadingZerosToken::new(0, 1),   // AC symbol: 0b00000001 x
                    LeadingZerosToken::new(0, 0),   // AC symbol: 0b00000000 x
                ],
            ),
        ];

        let expected: HuffmanCount = HuffmanCount {
            dc_count: vec![
                SymbolFrequency::new(5, 1),
                SymbolFrequency::new(0, 1),
                SymbolFrequency::new(6, 1),
                SymbolFrequency::new(1, 1),
            ],
            ac_count: vec![
                SymbolFrequency::new(0b00001001, 1),
                SymbolFrequency::new(0b11110000, 4),
                SymbolFrequency::new(0b01000011, 1),
                SymbolFrequency::new(0b00000000, 4),
                SymbolFrequency::new(0b00001010, 2),
                SymbolFrequency::new(0b01000100, 1),
                SymbolFrequency::new(0b00000111, 1),
                SymbolFrequency::new(0b00100011, 1),
                SymbolFrequency::new(0b00000001, 1),
            ],
        };

        let test_blocks_iterator = test_blocks_channel_1
            .iter()
            .chain(test_blocks_channel_2.iter());

        let actual = HuffmanCount::from_iter(test_blocks_iterator);

        for symfreq in actual.dc_count.iter() {
            let mut found = false;
            for comp in expected.dc_count.iter() {
                if symfreq.symbol == comp.symbol {
                    assert_eq!(symfreq.frequency, comp.frequency);
                    found = true;
                }
            }
            assert!(found);
        }

        for symfreq in actual.ac_count.iter() {
            let mut found = false;
            for comp in expected.ac_count.iter() {
                if symfreq.symbol == comp.symbol {
                    assert_eq!(symfreq.frequency, comp.frequency);
                    found = true;
                }
            }
            assert!(found);
        }
    }
}

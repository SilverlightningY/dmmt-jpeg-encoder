use super::frequency_block::FrequencyBlock;

#[derive(Clone, Copy)]
pub struct CategoryEncodedInteger {
    pub pattern_length: u8,
    pub pattern: u16,
}

impl CategoryEncodedInteger {
    fn get_category_of(value: i16) -> u8 {
        if value == 0 {
            return 0;
        }
        let absolute_value = value.unsigned_abs();
        let category = i16::BITS - absolute_value.leading_zeros();
        if category > 15 {
            panic!(
                "Unable to categorize value '{}' becaues it is out of range",
                value
            );
        }
        category as u8
    }

    fn calculate_pattern_of(value: i16, category: u8) -> u16 {
        if value == 0 {
            return 0;
        }
        if value.is_positive() {
            value.unsigned_abs()
        } else {
            let category_border_marker = 1 << (category - 1);
            2 * category_border_marker - 1 - value.unsigned_abs()
        }
    }

    fn left_align_pattern(pattern: u16, category: u8) -> u16 {
        if category == 0 {
            return 0;
        }
        let free_bits_in_pattern = u16::BITS as u8 - category;
        pattern << free_bits_in_pattern
    }
}

impl From<i16> for CategoryEncodedInteger {
    fn from(value: i16) -> Self {
        let category = Self::get_category_of(value);
        let pattern = Self::calculate_pattern_of(value, category);
        let pattern = Self::left_align_pattern(pattern, category);
        CategoryEncodedInteger {
            pattern_length: category,
            pattern,
        }
    }
}

pub struct LeadingZerosToken {
    zeros_before: u8,
    category: CategoryEncodedInteger,
}

impl LeadingZerosToken {
    pub fn new(zeros_before: u8, symbol: i16) -> Self {
        Self {
            // numbers of zeros before symbol
            zeros_before,
            category: CategoryEncodedInteger::from(symbol),
        }
    }

    pub fn combined_symbol(&self) -> u8 {
        let left_part = self.zeros_before << 4;
        let right_part = self.category.pattern_length;
        left_part | right_part
    }

    pub fn category(&self) -> CategoryEncodedInteger {
        self.category
    }
}

pub struct CategorizedBlock {
    dc_category: CategoryEncodedInteger,
    ac_tokens: Vec<LeadingZerosToken>,
}

impl CategorizedBlock {
    pub fn new(dc_category: CategoryEncodedInteger, ac_tokens: Vec<LeadingZerosToken>) -> Self {
        Self {
            dc_category,
            ac_tokens,
        }
    }

    pub fn iter_ac_symbols<'a>(&'a self) -> impl Iterator<Item = u8> + use<'a> {
        self.ac_tokens.iter().map(|t| t.combined_symbol())
    }

    pub fn dc_symbol(&self) -> u8 {
        self.dc_category.pattern_length
    }
}

fn sum_zeros_before_values<'a, T: Iterator<Item = &'a i16>>(sequence: T) -> Vec<LeadingZerosToken> {
    let mut result: Vec<LeadingZerosToken> = Vec::new();
    let mut zeros_encountered = 0;
    for &i in sequence {
        if i == 0 {
            zeros_encountered += 1;
        } else {
            while zeros_encountered > 15 {
                result.push(LeadingZerosToken::new(15, 0));
                zeros_encountered -= 16;
            }
            result.push(LeadingZerosToken::new(zeros_encountered, i));
            zeros_encountered = 0;
        }
    }
    if zeros_encountered != 0 {
        result.push(LeadingZerosToken::new(0, 0));
    }
    result
}

pub fn categorize_channel<T: Iterator<Item = FrequencyBlock<i16>>>(
    frequency_blocks: T,
) -> Vec<CategorizedBlock> {
    let mut categorized_blocks: Vec<CategorizedBlock> = Vec::new();
    let mut last_dc = 0;
    for frequency_block in frequency_blocks {
        let current_dc = *frequency_block.dc();
        let dc_category = CategoryEncodedInteger::from(current_dc - last_dc);
        last_dc = current_dc;
        let ac_components = sum_zeros_before_values(frequency_block.iter_zig_zag().skip(1));
        categorized_blocks.push(CategorizedBlock {
            ac_tokens: ac_components,
            dc_category,
        });
    }
    categorized_blocks
}

#[cfg(test)]
mod test {
    use super::{sum_zeros_before_values, CategoryEncodedInteger, LeadingZerosToken};

    #[test]
    fn test_categorize_integer() {
        let expected = [
            CategoryEncodedInteger {
                pattern_length: 6,
                pattern: 0b11100100_00000000u16,
            },
            CategoryEncodedInteger {
                pattern_length: 6,
                pattern: 0b10110100_00000000u16,
            },
            CategoryEncodedInteger {
                pattern_length: 1,
                pattern: 0b10000000_00000000u16,
            },
            CategoryEncodedInteger {
                pattern_length: 5,
                pattern: 0b00001000_00000000u16,
            },
        ];
        let input: Vec<i16> = vec![57, 45, 1, -30];
        for i in 0..4 {
            let v = input[i];
            let r = CategoryEncodedInteger::from(v);
            assert_eq!(
                expected[i].pattern_length, r.pattern_length,
                "Category/Bit pattern length does not match at index {}",
                i
            );
            assert_eq!(
                expected[i].pattern, r.pattern,
                "Bit pattern does not match at index {}",
                i
            );
        }
    }

    #[test]
    fn test_categorize_integer_max_value() {
        let expected_length = 15;
        let expected_pattern = 0b11111111_11111110;
        let actual = CategoryEncodedInteger::from(32767);
        assert_eq!(
            expected_length, actual.pattern_length,
            "Pattern length does not match"
        );
        assert_eq!(expected_pattern, actual.pattern, "Pattern does not match");
    }

    #[test]
    fn test_categorize_integer_min_value() {
        let expected_length = 15;
        let expected_pattern = 0;
        let actual = CategoryEncodedInteger::from(-32767);
        assert_eq!(
            expected_length, actual.pattern_length,
            "Pattern length does not match"
        );
        assert_eq!(expected_pattern, actual.pattern, "Pattern does not match");
    }

    #[test]
    #[should_panic]
    fn test_categorize_integer_lower_than_min_value() {
        let _ = CategoryEncodedInteger::from(-32768);
    }

    #[test]
    fn test_categorize_integer_zero() {
        let expected_length = 0;
        let expected_pattern = 0;
        let actual = CategoryEncodedInteger::from(0);
        assert_eq!(
            expected_length, actual.pattern_length,
            "Pattern length does not match"
        );
        assert_eq!(expected_pattern, actual.pattern, "Pattern does not match");
    }

    #[test]
    fn test_sum_zeros_before_values() {
        let test_sequence: Vec<i16> = vec![
            57, 45, 0, 0, 0, 0, 23, 0, -30, -16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0,
        ];
        let expect_sequence: Vec<LeadingZerosToken> = vec![
            LeadingZerosToken::new(0, 57),
            LeadingZerosToken::new(0, 45),
            LeadingZerosToken::new(4, 23),
            LeadingZerosToken::new(1, -30),
            LeadingZerosToken::new(0, -16),
            LeadingZerosToken::new(15, 0),
            LeadingZerosToken::new(3, 1),
            LeadingZerosToken::new(0, 0),
        ];
        let got_sequence: Vec<LeadingZerosToken> = sum_zeros_before_values(test_sequence.iter());

        for i in 0..got_sequence.len() {
            assert_eq!(
                got_sequence[i].zeros_before, expect_sequence[i].zeros_before,
                "Number of zeros before symbol does not match at index {}",
                i
            );
            assert_eq!(
                got_sequence[i].category.pattern_length, expect_sequence[i].category.pattern_length,
                "Category/Bit pattern length does not match at index {}",
                i
            );
            assert_eq!(
                got_sequence[i].category.pattern, expect_sequence[i].category.pattern,
                "Bit pattern does not match at index {}",
                i
            );
        }
    }
}

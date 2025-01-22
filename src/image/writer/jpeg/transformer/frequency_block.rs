const ZIG_ZAG_ORDERED_BLOCK_INDEXES: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

#[derive(Clone, Copy)]
pub struct FrequencyBlock<T> {
    data: [T; 64],
}

impl<T> FrequencyBlock<T> {
    pub fn new(data: [T; 64]) -> Self {
        Self { data }
    }

    pub fn iter_zig_zag(&self) -> ZigZagIterator<'_, T> {
        ZigZagIterator::from(self)
    }

    pub fn dc(&self) -> &T {
        &self.data[0]
    }
}

pub struct ZigZagIterator<'a, T> {
    data: &'a [T; 64],
    next_index: usize,
}

impl<'a, T> From<&'a FrequencyBlock<T>> for ZigZagIterator<'a, T> {
    fn from(block: &'a FrequencyBlock<T>) -> Self {
        Self {
            data: &block.data,
            next_index: 0,
        }
    }
}

impl<'a, T> From<&'a [T; 64]> for ZigZagIterator<'a, T> {
    fn from(data: &'a [T; 64]) -> Self {
        Self {
            data,
            next_index: 0,
        }
    }
}

impl<'a, T> Iterator for ZigZagIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= ZIG_ZAG_ORDERED_BLOCK_INDEXES.len() {
            return None;
        }
        let block_index = ZIG_ZAG_ORDERED_BLOCK_INDEXES[self.next_index];
        let return_value = &self.data[block_index];
        self.next_index += 1;
        Some(return_value)
    }
}

#[cfg(test)]
mod test {
    use super::FrequencyBlock;

    #[rustfmt::skip]
    const TEST_BLOCK_DATA_1: [usize; 64] = [
         0,  1,  5,  6, 14, 15, 27, 28,
         2,  4,  7, 13, 16, 26, 29, 42,
         3,  8, 12, 17, 25, 30, 41, 43,
         9, 11, 18, 24, 31, 40, 44, 53,
        10, 19, 23, 32, 39, 45, 52, 54,
        20, 22, 33, 38, 46, 51, 55, 60,
        21, 34, 37, 47, 50, 56, 59, 61,
        35, 36, 48, 49, 57, 58, 62, 63,
    ];

    #[test]
    fn test_frequency_block_iter_zig_zag() {
        let block = FrequencyBlock::new(TEST_BLOCK_DATA_1);
        for (expected, &actual) in block.iter_zig_zag().enumerate() {
            assert_eq!(
                expected, actual,
                "Data item at index {} does not match",
                expected
            );
        }
    }

    #[test]
    fn test_frequency_block_iter_zig_zag_count_is_64() {
        let block = FrequencyBlock::new(TEST_BLOCK_DATA_1);
        let actual = block.iter_zig_zag().count();
        let expected = 64;
        assert_eq!(
            actual, expected,
            "Zig Zag Iterator must only return 64 values"
        );
    }
}

use super::Discrete8x8CosineTransformer;

pub struct SeparatedDiscrete8x8CosineTransformer;

const A: [f32; 64] = [
    0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    0.490_392_63,
    0.415_734_8,
    0.277_785_12,
    0.097_545_16,
    -0.097_545_16,
    -0.277_785_12,
    -0.415_734_8,
    -0.490_392_63,
    0.461_939_75,
    0.191_341_71,
    -0.191_341_71,
    -0.461_939_75,
    -0.461_939_75,
    -0.191_341_71,
    0.191_341_71,
    0.461_939_75,
    0.415_734_8,
    -0.097_545_16,
    -0.490_392_63,
    -0.277_785_12,
    0.277_785_12,
    0.490_392_63,
    0.097_545_16,
    -0.415_734_8,
    0.353_553_38,
    -0.353_553_38,
    -0.353_553_38,
    0.353_553_38,
    0.353_553_38,
    -0.353_553_38,
    -0.353_553_38,
    0.353_553_38,
    0.277_785_12,
    -0.490_392_63,
    0.097_545_16,
    0.415_734_8,
    -0.415_734_8,
    -0.097_545_16,
    0.490_392_63,
    -0.277_785_12,
    0.191_341_71,
    -0.461_939_75,
    0.461_939_75,
    -0.191_341_71,
    -0.191_341_71,
    0.461_939_75,
    -0.461_939_75,
    0.191_341_71,
    0.097_545_16,
    -0.277_785_12,
    0.415_734_8,
    -0.490_392_63,
    0.490_392_63,
    -0.415_734_8,
    0.277_785_12,
    -0.097_545_16,
];

impl Discrete8x8CosineTransformer for SeparatedDiscrete8x8CosineTransformer {
    unsafe fn transform(&self, block_start_in: *const f32, block_start_out: *mut f32) {
        let mut intermediate: [f32; 64] = [0.0; 64];
        for i in 0..8 {
            for j in 0..8 {
                let mut acc: f32 = 0.0;
                for k in 0..8 {
                    acc += A[i * 8 + k] * *block_start_in.add(k * 8 + j);
                }
                intermediate[i * 8 + j] = acc;
            }
        }
        for i in 0..8 {
            for j in 0..8 {
                let mut acc: f32 = 0.0;
                for k in 0..8 {
                    acc += intermediate[i * 8 + k] * A[j * 8 + k];
                }
                *block_start_out.add(i * 8 + j) = acc;
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::cosine_transform::simple::InverseSimpleDiscrete8x8CosineTransformer;

    use super::super::Discrete8x8CosineTransformer;
    use super::SeparatedDiscrete8x8CosineTransformer;

    const TEST_BLOCK: [f32; 64] = [
        0.736259, 0.2606891, 0.5610827, 0.8214362, 0.9691457, 0.8678548, 0.6238593, 0.5084994,
        0.8050782, 0.7121189, 0.5455183, 0.9727164, 0.5572985, 0.2453382, 0.8806421, 0.1258583,
        0.8396557, 0.3285012, 0.348796, 0.7314371, 0.3823053, 0.5750602, 0.5600756, 0.7767876,
        0.3731192, 0.0588091, 0.6840113, 0.3082369, 0.1330607, 0.4003418, 0.9928281, 0.6752525,
        0.2386547, 0.1788079, 0.2037415, 0.320719, 0.0138248, 0.8993194, 0.5502792, 0.8301034,
        0.461806, 0.2384105, 0.3627735, 0.582995, 0.2926725, 0.9669484, 0.4517349, 0.7738883,
        0.2172307, 0.6634418, 0.6780297, 0.3852351, 0.2001098, 0.6359752, 0.8304086, 0.3636585,
        0.3370769, 0.1292153, 0.7361369, 0.9847407, 0.7540513, 0.5663624, 0.7456282, 0.474166,
    ];

    fn assert_eq_with_deviation(actual: f32, expected: f32, deviation: f32, index: usize) {
        let difference = actual - expected;
        let abs_difference = difference.abs();
        let comp_word = if difference.is_sign_positive() {
            "greater"
        } else {
            "smaller"
        };

        assert!(
            abs_difference < deviation,
            "Value {} at index {} is {} than {} with deviation of {}",
            actual,
            index,
            comp_word,
            expected,
            deviation
        );
    }

    fn assert_values_not_zero<'a>(values: impl IntoIterator<Item = &'a f32>) {
        for (index, &value) in values.into_iter().enumerate() {
            assert_ne!(value, 0_f32, "Value at index {} must not be zero", index);
        }
    }

    #[test]
    fn test_transform_to_frequency_domain_and_back() {
        let deviation = 1e-6_f32;
        let test_block = TEST_BLOCK;
	let mut out_block: [f32; 64] = [0.0;64];
        unsafe {
            SeparatedDiscrete8x8CosineTransformer.transform(&raw const test_block[0],&raw mut out_block[0]);
            assert_values_not_zero(&test_block);
            InverseSimpleDiscrete8x8CosineTransformer.transform(&raw const test_block[0],&raw mut out_block[0]);
        }
        for (index, (actual, expected)) in test_block.into_iter().zip(TEST_BLOCK).enumerate() {
            assert_eq_with_deviation(actual, expected, deviation, index);
        }
    }
}

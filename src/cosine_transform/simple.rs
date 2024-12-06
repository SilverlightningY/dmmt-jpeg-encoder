use core::f32;

use super::Discrete8x8CosineTransformer;

const SQUARE_SIZE: usize = 8;
const NUMBER_OF_VALUES: usize = SQUARE_SIZE * SQUARE_SIZE;

fn calculate_consine_argument(a: usize, b: usize) -> f32 {
    ((2 * a + 1) as f32 * b as f32 * f32::consts::PI) / (2 * SQUARE_SIZE) as f32
}

fn calculate_factor_c(value: usize) -> f32 {
    if value == 0 {
        return f32::consts::FRAC_1_SQRT_2;
    }
    1_f32
}

pub struct SimpleDiscrete8x8CosineTransformer;

impl SimpleDiscrete8x8CosineTransformer {
    fn calculate_normalization_factor(i: usize, j: usize) -> f32 {
        2_f32 / SQUARE_SIZE as f32 * calculate_factor_c(i) * calculate_factor_c(j)
    }

    fn calculate_value(i: usize, j: usize, input_values: &[f32; NUMBER_OF_VALUES]) -> f32 {
        Self::calculate_normalization_factor(i, j) * Self::sum_up_cosines(i, j, input_values)
    }

    fn sum_up_cosines(i: usize, j: usize, input_values: &[f32; NUMBER_OF_VALUES]) -> f32 {
        input_values
            .iter()
            .enumerate()
            .map(|(index, &input_value)| {
                let x = index % SQUARE_SIZE;
                let y = index / SQUARE_SIZE;
                input_value
                    * calculate_consine_argument(x, i).cos()
                    * calculate_consine_argument(y, j).cos()
            })
            .sum()
    }
}

impl Discrete8x8CosineTransformer for SimpleDiscrete8x8CosineTransformer {
    fn transform(values: &[f32; NUMBER_OF_VALUES]) -> [f32; NUMBER_OF_VALUES] {
        (0..NUMBER_OF_VALUES)
            .map(|index| {
                let i = index % SQUARE_SIZE;
                let j = index / SQUARE_SIZE;
                (index, Self::calculate_value(i, j, values))
            })
            .fold(
                [f32::default(); NUMBER_OF_VALUES],
                |mut acc, (index, value)| {
                    acc[index] = value;
                    acc
                },
            )
    }
}

pub struct InverseSimpleDiscrete8x8CosineTransformer;

impl InverseSimpleDiscrete8x8CosineTransformer {
    fn sum_up_inner_product(x: usize, y: usize, values: &[f32; NUMBER_OF_VALUES]) -> f32 {
        values
            .iter()
            .enumerate()
            .map(|(index, &value)| {
                let i = index % SQUARE_SIZE;
                let j = index / SQUARE_SIZE;
                value
                    * calculate_factor_c(i)
                    * calculate_factor_c(j)
                    * calculate_consine_argument(x, i).cos()
                    * calculate_consine_argument(y, j).cos()
            })
            .sum()
    }

    fn calculate_value(x: usize, y: usize, values: &[f32; NUMBER_OF_VALUES]) -> f32 {
        (2_f32 / SQUARE_SIZE as f32) * Self::sum_up_inner_product(x, y, values)
    }
}

impl Discrete8x8CosineTransformer for InverseSimpleDiscrete8x8CosineTransformer {
    fn transform(values: &[f32; 64]) -> [f32; 64] {
        (0..NUMBER_OF_VALUES)
            .map(|index| {
                let x = index % SQUARE_SIZE;
                let y = index / SQUARE_SIZE;
                (index, Self::calculate_value(x, y, values))
            })
            .fold(
                [f32::default(); NUMBER_OF_VALUES],
                |mut acc, (index, value)| {
                    acc[index] = value;
                    acc
                },
            )
    }
}

#[cfg(test)]
mod test {
    use super::super::Discrete8x8CosineTransformer;
    use super::{InverseSimpleDiscrete8x8CosineTransformer, SimpleDiscrete8x8CosineTransformer};

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
        assert!(
            actual <= expected + deviation,
            "Value {} at index {} is greater than {} with deviation of {}",
            actual,
            index,
            expected,
            deviation
        );
        assert!(
            actual >= expected - deviation,
            "Value {} at index {} is smaller than {} with deviation of {}",
            actual,
            index,
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
        let frequencies = SimpleDiscrete8x8CosineTransformer::transform(&TEST_BLOCK);
        assert_values_not_zero(&frequencies);
        let colors = InverseSimpleDiscrete8x8CosineTransformer::transform(&frequencies);
        for (index, (actual, expected)) in colors.into_iter().zip(TEST_BLOCK).enumerate() {
            assert_eq_with_deviation(actual, expected, deviation, index);
        }
    }
}

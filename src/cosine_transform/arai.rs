use std::f32::consts::{FRAC_1_SQRT_2, PI};

use super::Discrete8x8CosineTransformer;

pub struct AraiDiscrete8x8CosineTransformer {}

impl Discrete8x8CosineTransformer for AraiDiscrete8x8CosineTransformer {
    fn transform(values: &[f32; 64]) -> [f32; 64] {
        let mut row_pass: Vec<f32> = Vec::new();
        for group in values.chunks_exact(8) {
            row_pass.append(&mut Self::apply_arai(group.try_into().unwrap()).to_vec());
        }
        let mut column_pass: Vec<f32> = Vec::new();
        let rotated = rotate_matrix(&mut row_pass);
        for group in rotated.chunks_exact(8) {
            column_pass.append(&mut Self::apply_arai(group.try_into().unwrap()).to_vec());
        }
        column_pass.try_into().unwrap()
    }
}

fn rotate_matrix(input: &mut [f32]) -> Vec<f32> {
    let mut rotated: Vec<f32> = Vec::new();
    let mut idx = 0;
    (0..8).for_each(|i| {
        idx = i;
        while idx < input.len() {
            rotated.push(input[idx]);
            idx += 8
        }
    });
    log::warn!("len: {}", rotated.len());
    rotated
}

type Row = [f32; 8];

impl AraiDiscrete8x8CosineTransformer {
    fn apply_arai(inputs: &Row) -> Row {
        let step1 = Self::step1_cross(inputs);
        let step2 = Self::step2_cross_lines(&step1);
        let mut step3 = Self::step3_top4(&step2);
        let step4 = Self::step4_multiplications(&mut step3);
        let step5 = Self::step5_some_crosses(step4);
        let mut step6 = Self::step6_bottom4(step5);
        Self::step7_final_multiplication(&mut step6)
    }

    fn step1_cross(input: &Row) -> Row {
        let mut output = [0.0; 8];
        output[0] = input[0] + input[7];
        output[1] = input[1] + input[6];
        output[2] = input[2] + input[5];
        output[3] = input[3] + input[4];
        output[4] = -input[4] + input[3];
        output[5] = -input[5] + input[2];
        output[6] = -input[6] + input[1];
        output[7] = -input[7] + input[0];
        output
    }

    fn step2_cross_lines(input: &Row) -> Row {
        let mut output = [0.0; 8];
        output[0] = input[0] + input[3];
        output[1] = input[1] + input[2];
        output[2] = -input[2] + input[1];
        output[3] = -input[3] + input[0];
        output[4] = -input[4] - input[5];
        output[5] = input[5] + input[6];
        output[6] = input[6] + input[7];
        output[7] = input[7];
        output
    }

    fn step3_top4(input: &Row) -> Row {
        let mut output = [0.0; 8];
        output[0] = input[0] + input[1];
        output[1] = -input[1] + input[0];
        output[2] = input[2] + input[3];
        output[3] = input[3];
        output[4] = input[4];
        output[5] = input[5];
        output[6] = input[6];
        output[7] = input[7];
        output
    }

    const A1: f32 = FRAC_1_SQRT_2;
    // sqrt(1 - FRAC_1_SQRT_2)
    const A2: f32 = 0.5411961;
    const A3: f32 = Self::A1;
    // sqrt(1 + FRAC_1_SQRT_2)
    const A4: f32 = 1.3065629;
    // 1/2 * sqrt(2 - √2)
    const A5: f32 = 0.3826834;

    // 1 / 2 * sqrt(2)
    const S0: f32 = 0.3535533;
    // From here Sk = 1 / 4*Ck
    // Ck = cos(PI*k/16)
    const S1: f32 = 0.1767766;
    const S2: f32 = 0.24519631;
    const S3: f32 = 0.23096988;
    const S4: f32 = 0.2078674;
    const S5: f32 = 0.17677669;
    const S6: f32 = 0.13889255;
    const S7: f32 = 0.0;

    #[rustfmt::skip]
    const S_VALUES: &[f32; 8] = &[Self::S0, Self::S4, Self::S2, Self::S5, Self::S6, Self::S1, Self::S7, Self::S3];

    fn compute_manually_s(k: u8) -> f32 {
        f32::cos(PI * k as f32 / 16_f32) * 0.25
    }

    fn step4_multiplications(input: &mut Row) -> Row {
        input[2] *= Self::A1 * -Self::A5;
        input[4] *= -Self::A2;
        input[5] *= Self::A3;
        input[6] *= Self::A4 * -Self::A5;
        *input
    }

    fn step5_some_crosses(input: Row) -> Row {
        let mut output = [0.0; 8];
        output[0] = input[0];
        output[1] = input[1];
        output[2] = input[2] + input[3];
        output[3] = input[3] - input[2];
        output[4] = input[4];
        output[5] = input[5] + input[7];
        output[6] = input[6];
        output[7] = input[7] - input[5];
        output
    }

    fn step6_bottom4(input: Row) -> Row {
        let mut output = [0.0; 8];
        output[0] = input[0];
        output[1] = input[1];
        output[2] = input[2];
        output[3] = input[3];
        output[4] = input[4] + input[7];
        output[5] = input[5] + input[6];
        output[6] = -input[6] + input[5];
        output[7] = input[7] - input[4];
        output
    }

    fn step7_final_multiplication(input: &mut Row) -> Row {
        (0..8).for_each(|i| input[i] *= Self::S_VALUES[i]);
        input.to_owned()
    }
}

mod test {
    use super::AraiDiscrete8x8CosineTransformer;
    use crate::cosine_transform::{arai::rotate_matrix, Discrete8x8CosineTransformer};

    #[rustfmt::skip]
    const TEST_VALUES: [f32; 64] = [
        1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 3.0, 2.0,
        3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0,
        3.0, 4.0, 3.0, 2.0, 3.0, 4.0, 5.0, 6.0,
        7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 3.0, 2.0,
        3.0, 4.0, 5.0, 5.0, 6.0, 5.0, 2.0, 3.0,
        4.0, 3.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0,
        2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 
        2.0, 3.0, 4.0, 5.0, 3.0, 4.0, 3.0, 4.0,
    ];

    #[rustfmt::skip]
    const TEST_VALUES_ROTATED: [f32; 64] = [
        1.0, 3.0, 3.0, 7.0, 3.0, 4.0, 2.0, 2.0,
        2.0, 2.0, 4.0, 6.0, 4.0, 3.0, 3.0, 3.0,
        1.0, 1.0, 3.0, 5.0, 5.0, 2.0, 4.0, 4.0,
        2.0, 2.0, 2.0, 4.0, 5.0, 3.0, 5.0, 5.0,
        3.0, 3.0, 3.0, 3.0, 6.0, 4.0, 6.0, 3.0,
        2.0, 4.0, 4.0, 2.0, 5.0, 5.0, 5.0, 4.0,
        3.0, 3.0, 5.0, 3.0, 2.0, 4.0, 4.0, 3.0, 
        2.0, 2.0, 6.0, 2.0, 3.0, 3.0, 3.0, 4.0,
    ];

    #[test]
    fn test_transform() {
        let new = AraiDiscrete8x8CosineTransformer::transform(&TEST_VALUES);
        log::warn!("{:?}", new)
    }

    #[test]
    fn test_matrix_rotation() {
        assert_eq!(
            TEST_VALUES_ROTATED.to_vec(),
            rotate_matrix(&mut TEST_VALUES.to_vec())
        )
    }

    #[test]
    fn compute_manually() {
        (0..6).for_each(|i| {
            log::warn!(
                "{}",
                AraiDiscrete8x8CosineTransformer::compute_manually_s(i)
            )
        });
    }
}

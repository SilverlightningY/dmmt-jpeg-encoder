use std::f32::consts::FRAC_1_SQRT_2;

use super::Discrete8x8CosineTransformer;

pub struct AraiDiscrete8x8CosineTransformer;

const A1: f32 = FRAC_1_SQRT_2;
// sqrt(1 - FRAC_1_SQRT_2)
const A2: f32 = 0.5411961;
const A3: f32 = A1;
// sqrt(1 + FRAC_1_SQRT_2)
const A4: f32 = 1.3065629;
// 1/2 * sqrt(2 - √2)
const A5: f32 = 0.3826834;

// 1 / 2 * sqrt(2)
const S0: f32 = 0.3535533;
// From here Sk = 1 / 4*Ck
// Ck = cos(PI*k/16)
const S1: f32 = 0.254_897_8;
const S2: f32 = 0.270_598_05;
const S3: f32 = 0.300_672_44;
const S4: f32 = 0.353_553_38;
const S5: f32 = 0.449_988_1;
const S6: f32 = 0.653_281_5;
const S7: f32 = 1.281_457_7;

impl AraiDiscrete8x8CosineTransformer {
    fn fast_arai(inputs: &mut [f32], stride: usize) {
        let v00 = inputs[0];
        let v01 = inputs[stride];
        let v02 = inputs[2 * stride];
        let v03 = inputs[3 * stride];
        let v04 = inputs[4 * stride];
        let v05 = inputs[5 * stride];
        let v06 = inputs[6 * stride];
        let v07 = inputs[7 * stride];

        let v10 = v00 + v07;
        let v11 = v01 + v06;
        let v12 = v02 + v05;
        let v13 = v03 + v04;
        let v14 = v03 - v04;
        let v15 = v02 - v05;
        let v16 = v01 - v06;
        let v17 = v00 - v07;

        let v20 = v10 + v13;
        let v21 = v11 + v12;
        let v22 = v11 - v12;
        let v23 = v10 - v13;
        let v24 = -v14 - v15;
        let v25 = v15 + v16;
        let v26 = v16 + v17;

        let v30 = v20 + v21;
        let v31 = v20 - v21;
        let v32 = v22 + v23;

        let v42 = v32 * A1;
        let v44 = -v24 * A2 - (v24 + v26) * A5;
        let v45 = v25 * A3;
        let v46 = v26 * A4 - (v26 + v24) * A5;

        let v52 = v42 + v23;
        let v53 = v23 - v42;
        let v55 = v45 + v17;
        let v57 = v17 - v45;

        let v64 = v44 + v57;
        let v65 = v55 + v46;
        let v66 = v55 - v46;
        let v67 = v57 - v44;

        inputs[0] = v30 * S0;
        inputs[4 * stride] = v31 * S4;
        inputs[2 * stride] = v52 * S2;
        inputs[6 * stride] = v53 * S6;
        inputs[5 * stride] = v64 * S5;
        inputs[stride] = v65 * S1;
        inputs[7 * stride] = v66 * S7;
        inputs[3 * stride] = v67 * S3;
    }
}

impl Discrete8x8CosineTransformer for AraiDiscrete8x8CosineTransformer {
    fn transform(&self, image_slice: &mut [f32], row_lenght: usize) {
        for i in 0..8 {
            Self::fast_arai(&mut image_slice[i * row_lenght..], 1)
        }
        for i in 0..8 {
            Self::fast_arai(&mut image_slice[i..], row_lenght);
        }
    }
}

#[cfg(test)]
mod test {

    use super::super::Discrete8x8CosineTransformer;
    use super::{
        AraiDiscrete8x8CosineTransformer, A1, A2, A3, A4, A5, S0, S1, S2, S3, S4, S5, S6, S7,
    };

    type Row = [f32; 8];

    fn y0(input: &Row) -> f32 {
        let s: f32 = input.iter().sum();
        s * S0
    }

    fn y4(i: &Row) -> f32 {
        (i[0] + i[7] + i[3] + i[4] - i[1] - i[6] - i[2] - i[5]) * S4
    }

    fn y2(i: &Row) -> f32 {
        ((i[0] + i[1] - i[2] - i[3] - i[4] - i[5] + i[6] + i[7]) * A1 + i[0] + i[7] - i[3] - i[4])
            * S2
    }

    fn y6(i: &Row) -> f32 {
        ((i[0] + i[1] - i[2] - i[3] - i[4] - i[5] + i[6] + i[7]) * -A1 + i[0] + i[7] - i[3] - i[4])
            * S6
    }

    fn y5(i: &Row) -> f32 {
        (A2 * (i[3] - i[4] + i[2] - i[5])
            + A5 * (i[3] - i[4] + i[2] - i[5] - i[1] + i[6] - i[0] + i[7])
            + i[0]
            - i[7]
            - A3 * (i[2] - i[5] + i[1] - i[6]))
            * S5
    }

    fn y1(i: &Row) -> f32 {
        let temp = i[1] - i[6] + i[0] - i[7];
        (i[0] - i[7] + A3 * (i[2] - i[5] + i[1] - i[6]) + A4 * (temp)
            - A5 * (temp - i[3] + i[4] - i[0] + i[7]))
            * S1
    }

    fn y7(i: &Row) -> f32 {
        let temp = i[1] - i[6] + i[0] - i[7];
        (i[0] - i[7] + A3 * (i[2] - i[5] + i[1] - i[6]) - A4 * (temp)
            + A5 * (temp - i[3] + i[4] - i[0] + i[7]))
            * S7
    }

    fn y3(i: &Row) -> f32 {
        let temp = i[3] - i[4] + i[2] - i[5];
        (-A2 * temp - A5 * (temp - i[1] + i[6] - i[0] + i[7]) + i[0]
            - i[7]
            - A3 * (i[2] - i[5] + i[1] - i[6]))
            * S3
    }

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

    fn assert_almost_eq(actual: f32, expected: f32, deviation: f32, index: usize) {
        assert!(
            (expected - actual).abs() <= deviation,
            "Value {} at index {} is different than {} with deviation of {}",
            actual,
            index,
            expected,
            deviation
        );
    }

    #[ignore]
    #[test]
    fn test_fast_simple() {
        let mut input: [f32; 64] = [0.0; 64]; // Initialize a mutable array with default values
        input.copy_from_slice(&TEST_VALUES);

        AraiDiscrete8x8CosineTransformer.transform(&mut input, 8);
        let mut input2 = TEST_VALUES;
        //SimpleDiscrete8x8CosineTransformer.transform(&mut input2);
        for i in 0..64 {
            assert_almost_eq(input[i], input2[i], 1e-4, i)
        }
    }

    #[test]
    fn compare_fast_own() {
        let mut input = TEST_VALUES;
        AraiDiscrete8x8CosineTransformer::fast_arai(&mut input, 1);
        let mut input2: [f32; 8] = [0.0; 8];
        input2.copy_from_slice(&TEST_VALUES[0..8]);
        assert_eq!(input[0], y0(&input2), "Wrong Y0 calculated");
        assert_eq!(input[4], y4(&input2), "Wrong Y4 calculated");
        assert_eq!(input[2], y2(&input2), "Wrong Y2 calculated");
        assert_eq!(input[6], y6(&input2), "Wrong Y6 calculated");
        assert_eq!(input[5], y5(&input2), "Wrong Y5 calculated");
        assert_eq!(input[1], y1(&input2), "Wrong Y1 calculated");
        assert_eq!(input[7], y7(&input2), "Wrong Y7 calculated");
        assert_eq!(input[3], y3(&input2), "Wrong Y3 calculated");
    }
}

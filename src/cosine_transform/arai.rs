use std::f32::consts::{FRAC_1_SQRT_2, PI};

use super::Discrete8x8CosineTransformer;

pub struct AraiDiscrete8x8CosineTransformer {}

impl Discrete8x8CosineTransformer for AraiDiscrete8x8CosineTransformer {
    fn transform(values: &[f32; 64]) -> [f32; 64] {
        let mut row_pass: [f32; 64] = [0.0; 64];
        let mut counter = 0;
        for group in values.chunks_exact(8) {
            let mut row: [f32; 8] = group.try_into().expect("Slice with incorrect length");
            Self::fast_arai(&mut row);
            (0..8).for_each(|i| {
                row_pass[counter] = row[i];
                counter += 1;
            });
        }
        println!("{:?}", row_pass);
        let mut column_pass: [f32; 64] = [0.0; 64];
        for col in 0..8 {
            let mut group = [0.0; 8];
            let mut index = [0; 8];
            for row in 0..8 {
                let idx = row * 8 + col;
                group[row] = row_pass[idx];
                index[row] = idx;
            }
            println!("{:?}", group);
            println!("{:?}", index);

            Self::fast_arai(&mut group);
            for i in 0..8 {
                column_pass[index[i]] = group[i];
            }
        }
        column_pass
    }
}

type Row = [f32; 8];

impl AraiDiscrete8x8CosineTransformer {
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

    fn fast_arai(inputs: &mut Row) {
        let v10 = inputs[0] + inputs[7];
        let v11 = inputs[1] + inputs[6];
        let v12 = inputs[2] + inputs[5];
        let v13 = inputs[3] + inputs[4];
        let v14 = inputs[3] - inputs[4];
        let v15 = inputs[2] - inputs[5];
        let v16 = inputs[1] - inputs[6];
        let v17 = inputs[0] - inputs[7];

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

        let v42 = v32 * Self::A1;
        let v44 = -v24 * Self::A2 - v24 * Self::A5;
        let v45 = v25 * Self::A3;
        let v46 = v26 * Self::A4 - v26 * Self::A5;

        let v52 = v42 + v23;
        let v53 = v23 - v42;
        let v55 = v45 + v17;
        let v57 = v17 - v45;

        let v64 = v44 + v57;
        let v65 = v55 + v46;
        let v66 = v55 - v46;
        let v67 = v57 - v44;

        inputs[0] = v30 * Self::S0;
        inputs[4] = v31 * Self::S4;
        inputs[2] = v52 * Self::S2;
        inputs[6] = v53 * Self::S6;
        inputs[5] = v64 * Self::S5;
        inputs[1] = v65 * Self::S1;
        inputs[7] = v66 * Self::S7;
        inputs[3] = v67 * Self::S3;
    }

    fn y0(input: &Row) -> f32 {
        let s: f32 = input.iter().sum();
        s * Self::S_VALUES[0]
    }

    fn y4(i: &Row) -> f32 {
        (i[0] + i[7] + i[3] + i[4] - i[1] - i[6] - i[2] - i[5]) * Self::S_VALUES[4]
    }

    fn y2(i: &Row) -> f32 {
        ((i[0] + i[1] - i[2] - i[3] - i[4] - i[5] + i[6] + i[7]) * Self::A1 + i[0] + i[7]
            - i[3]
            - i[4])
            * Self::S_VALUES[2]
    }

    fn y6(i: &Row) -> f32 {
        ((i[0] + i[1] - i[2] - i[3] - i[4] - i[5] + i[6] + i[7]) * -Self::A1 + i[0] + i[7]
            - i[3]
            - i[4])
            * Self::S_VALUES[6]
    }

    fn y5(i: &Row) -> f32 {
        (Self::A2 * (i[3] - i[4] + i[2] - i[5]) + Self::A5 * (i[3] - i[4] + i[2] - i[5]) + i[0]
            - i[7]
            - Self::A3 * (i[2] - i[5] + i[1] - i[6]))
            * Self::S_VALUES[5]
    }

    fn y1(i: &Row) -> f32 {
        let temp = i[1] - i[6] + i[0] - i[7];
        (i[0] - i[7] + Self::A3 * (i[2] - i[5] + i[1] - i[6]) + Self::A4 * (temp)
            - Self::A5 * (temp))
            * Self::S_VALUES[1]
    }

    fn y7(i: &Row) -> f32 {
        let temp = i[1] - i[6] + i[0] - i[7];
        (i[0] - i[7] + Self::A3 * (i[2] - i[5] + i[1] - i[6]) - Self::A4 * (temp)
            + Self::A5 * (temp))
            * Self::S_VALUES[7]
    }

    fn y3(i: &Row) -> f32 {
        (-Self::A2 * (i[3] - i[4] + i[2] - i[5]) - Self::A5 * (i[3] - i[4] + i[2] - i[5]) + i[0]
            - i[7]
            - Self::A3 * (i[2] - i[5] + i[1] - i[6]))
            * Self::S_VALUES[3]
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
    const S1: f32 = 0.24519631;
    const S2: f32 = 0.23096988;
    const S3: f32 = 0.2078674;
    const S4: f32 = 0.17677669;
    const S5: f32 = 0.13889255;
    const S6: f32 = 0.09567086;
    const S7: f32 = 0.04877258;

    #[rustfmt::skip]
    const S_VALUES: &[f32; 8] = &[Self::S0, Self::S1, Self::S2,  Self::S3, Self::S4, Self::S5, Self::S6, Self::S7];
}

mod test {
    use std::f32::consts::PI;

    use crate::cosine_transform::Discrete8x8CosineTransformer;

    use super::AraiDiscrete8x8CosineTransformer;

    fn compute_manually_s(k: u8) -> f32 {
        f32::cos(PI * k as f32 / 16_f32) * 0.25
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
    fn test_calculated_s_values() {
        (1..7).for_each(|i| {
            assert_eq!(
                compute_manually_s(i),
                AraiDiscrete8x8CosineTransformer::S_VALUES[i as usize],
                "Computed value at index {} is not the same as stored value in constant",
                i
            )
        });
    }

    #[test]
    fn compare_fast_own() {
        let mut input: [f32; 8] = [0.0; 8]; // Initialize a mutable array with default values
        input.copy_from_slice(&TEST_VALUES[0..8]);
        AraiDiscrete8x8CosineTransformer::fast_arai(&mut input);
        let mut input2: [f32; 8] = [0.0; 8]; // Initialize a mutable array with default values
        input2.copy_from_slice(&TEST_VALUES[0..8]);
        assert_eq!(
            input[0],
            AraiDiscrete8x8CosineTransformer::y0(&input2),
            "Wrong Y0 calculated"
        );
        assert_eq!(
            input[4],
            AraiDiscrete8x8CosineTransformer::y4(&input2),
            "Wrong Y4 calculated"
        );
        assert_eq!(
            input[2],
            AraiDiscrete8x8CosineTransformer::y2(&input2),
            "Wrong Y2 calculated"
        );
        assert_eq!(
            input[6],
            AraiDiscrete8x8CosineTransformer::y6(&input2),
            "Wrong Y6 calculated"
        );
        assert_eq!(
            input[5],
            AraiDiscrete8x8CosineTransformer::y5(&input2),
            "Wrong Y5 calculated"
        );
        assert_eq!(
            input[1],
            AraiDiscrete8x8CosineTransformer::y1(&input2),
            "Wrong Y1 calculated"
        );
        assert_eq!(
            input[7],
            AraiDiscrete8x8CosineTransformer::y7(&input2),
            "Wrong Y7 calculated"
        );
        assert_eq!(
            input[3],
            AraiDiscrete8x8CosineTransformer::y3(&input2),
            "Wrong Y3 calculated"
        );
    }
}

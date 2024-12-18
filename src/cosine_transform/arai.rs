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
    unsafe fn fast_arai(block_start_in: *const f32, block_start_out: *mut f32, stride: usize) {
        let p0 = block_start_in;
        let p1 = block_start_in.add(stride);
        let p2 = block_start_in.add(2 * stride);
        let p3 = block_start_in.add(3 * stride);
        let p4 = block_start_in.add(4 * stride);
        let p5 = block_start_in.add(5 * stride);
        let p6 = block_start_in.add(6 * stride);
        let p7 = block_start_in.add(7 * stride);

        let v00 = *p0;
        let v01 = *p1;
        let v02 = *p2;
        let v03 = *p3;
        let v04 = *p4;
        let v05 = *p5;
        let v06 = *p6;
        let v07 = *p7;

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

	let op0 = block_start_out;
        let op1 = block_start_out.add(stride);
        let op2 = block_start_out.add(2 * stride);
        let op3 = block_start_out.add(3 * stride);
        let op4 = block_start_out.add(4 * stride);
        let op5 = block_start_out.add(5 * stride);
        let op6 = block_start_out.add(6 * stride);
        let op7 = block_start_out.add(7 * stride);
        *op0 = v30 * S0;
        *op4 = v31 * S4;
        *op2 = v52 * S2;
        *op6 = v53 * S6;
        *op5 = v64 * S5;
        *op1 = v65 * S1;
        *op7 = v66 * S7;
        *op3 = v67 * S3;
    }
}

impl Discrete8x8CosineTransformer for AraiDiscrete8x8CosineTransformer {
    unsafe fn transform(&self, block_start_in: *const f32, block_start_out: *mut f32) {
        for i in 0..8 {
            Self::fast_arai(block_start_in.add(i * 8),block_start_out.add(i * 8), 1)
        }
        for i in 0..8 {
            Self::fast_arai(block_start_out.add(i),    block_start_out.add(i), 8);
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::simple::SimpleDiscrete8x8CosineTransformer;
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

    #[test]
    fn test_fast_simple() {
        let test_values = TEST_VALUES;
	let mut out_test: [f32; 64] = [0.0;64];
	let mut out_simple: [f32; 64] = [0.0;64];

        unsafe {
            AraiDiscrete8x8CosineTransformer.transform(&raw const test_values[0], &raw mut out_test[0]);
            SimpleDiscrete8x8CosineTransformer.transform(&raw const test_values[0], &raw mut out_simple[0]);
        }
        for i in 0..64 {
            assert_almost_eq(out_test[i], out_simple[i], 1e-4, i)
        }
    }

    #[test]
    fn compare_fast_own() {
        let input = TEST_VALUES;
	let mut output: [f32;64] = [0.0;64];
        unsafe {
            AraiDiscrete8x8CosineTransformer::fast_arai(&raw const input[0], &raw mut output[0], 1);
        }
        let input2 = TEST_VALUES[0..8].try_into().unwrap();
        assert_eq!(output[0], y0(input2), "Wrong Y0 calculated");
        assert_eq!(output[4], y4(input2), "Wrong Y4 calculated");
        assert_eq!(output[2], y2(input2), "Wrong Y2 calculated");
        assert_eq!(output[6], y6(input2), "Wrong Y6 calculated");
        assert_eq!(output[5], y5(input2), "Wrong Y5 calculated");
        assert_eq!(output[1], y1(input2), "Wrong Y1 calculated");
        assert_eq!(output[7], y7(input2), "Wrong Y7 calculated");
        assert_eq!(output[3], y3(input2), "Wrong Y3 calculated");
    }
}

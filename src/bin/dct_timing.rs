use std::time::Instant;

use dmmt_jpeg_encoder::cosine_transform::{
    separated::SeparatedDiscrete8x8CosineTransformer, Discrete8x8CosineTransformer,
};

fn main() {
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
    let start = Instant::now();
    for _ in 0..40_000_000 {
        SeparatedDiscrete8x8CosineTransformer::transform(&TEST_BLOCK);
    }
    let duration = start.elapsed();
    println!("Time elapsed (return 256 bytes each time): {:?}", duration);

    let mut output: [f32; 64] = [0.0; 64];
    let start = Instant::now();
    for _ in 0..40_000_000 {
        SeparatedDiscrete8x8CosineTransformer::transform_without_return_copy(&TEST_BLOCK, & mut output);
    }
    let duration = start.elapsed();
    println!("Time elapsed (with return pointer): {:?}", duration);
}


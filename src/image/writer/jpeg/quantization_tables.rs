use clap::{builder::PossibleValue, ValueEnum};

use super::QuantizationTablePair;

// Tables from JPEG Annex K (vips and libjpeg default)
// JPEG Annex K
#[rustfmt::skip]
pub const SPECIFICATION_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] =  [
    16,  11,  10,  16,  24,  40,  51,  61,
    12,  12,  14,  19,  26,  58,  60,  55,
    14,  13,  16,  24,  40,  57,  69,  56,
    14,  17,  22,  29,  51,  87,  80,  62,
    18,  22,  37,  56,  68, 109, 103,  77,
    24,  35,  55,  64,  81, 104, 113,  92,
    49,  64,  78,  87, 103, 121, 120, 101,
    72,  92,  95,  98, 112, 100, 103,  99,
];

#[rustfmt::skip]
pub const SPECIFICATION_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    17,  18,  24,  47,  99,  99,  99,  99,
    18,  21,  26,  66,  99,  99,  99,  99,
    24,  26,  56,  99,  99,  99,  99,  99,
    47,  66,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
];

// Flat table
#[rustfmt::skip]
pub const FLAT_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
];

#[rustfmt::skip]
pub const FLAT_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
    16,  16,  16,  16,  16,  16,  16,  16,
];

// Table tuned for MSSIM on Kodak image set
#[rustfmt::skip]
pub const MSSIM_KODAK_TUNED_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    12,  17,  20,  21,  30,  34,  56,  63,
    18,  20,  20,  26,  28,  51,  61,  55,
    19,  20,  21,  26,  33,  58,  69,  55,
    26,  26,  26,  30,  46,  87,  86,  66,
    31,  33,  36,  40,  46,  96, 100,  73,
    40,  35,  46,  62,  81, 100, 111,  91,
    46,  66,  76,  86, 102, 121, 120, 101,
    68,  90,  90,  96, 113, 102, 105, 103,
];

#[rustfmt::skip]
pub const MSSIM_KODAK_TUNED_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
     8,  12,  15,  15,  86,  96,  96,  98,
    13,  13,  15,  26,  90,  96,  99,  98,
    12,  15,  18,  96,  99,  99,  99,  99,
    17,  16,  90,  96,  99,  99,  99,  99,
    96,  96,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
];

// Table from ImageMagick by N. Robidoux (current mozjpeg default)
// From http://www.imagemagick.org/discourse-server/viewtopic.php?f=22&t=20333&p=98008#p98008
// #[rustfmt::skip]
// pub const IMAGE_MAGICK_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
//     16,  16,   16,  18,  25,  37,  56,  85,
//     16,  17,   20,  27,  34,  40,  53,  75,
//     16,  20,   24,  31,  43,  62,  91, 135,
//     18,  27,   31,  40,  53,  74, 106, 156,
//     25,  34,   43,  53,  69,  94, 131, 189,
//     37,  40,   62,  74,  94, 124, 169, 238,
//     56,  53,   91, 106, 131, 169, 226, 311,
//     85,  75,  135, 156, 189, 238, 311, 418,
// ];
//
// #[rustfmt::skip]
// pub const IMAGE_MAGICK_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
//     16,  16,   16,  18,  25,  37,  56,  85,
//     16,  17,   20,  27,  34,  40,  53,  75,
//     16,  20,   24,  31,  43,  62,  91, 135,
//     18,  27,   31,  40,  53,  74, 106, 156,
//     25,  34,   43,  53,  69,  94, 131, 189,
//     37,  40,   62,  74,  94, 124, 169, 238,
//     56,  53,   91, 106, 131, 169, 226, 311,
//     85,  75,  135, 156, 189, 238, 311, 418,
// ];

// Table tuned for PSNR-HVS-M on Kodak image set
#[rustfmt::skip]
pub const PSNRHVSNI_KODAK_TUNED_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
     9,  10,  12,  14,  27,  32,  51,  62,
    11,  12,  14,  19,  27,  44,  59,  73,
    12,  14,  18,  25,  42,  59,  79,  78,
    17,  18,  25,  42,  61,  92,  87,  92,
    23,  28,  42,  75,  79, 112, 112,  99,
    40,  42,  59,  84,  88, 124, 132, 111,
    42,  64,  78,  95, 105, 126, 125,  99,
    70,  75, 100, 102, 116, 100, 107,  98,
];

#[rustfmt::skip]
pub const PSNRHVSNI_KODAK_TUNED_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
     9,  10,  17,  19,  62,  89,  91,  97,
    12,  13,  18,  29,  84,  91,  88,  98,
    14,  19,  29,  93,  95,  95,  98,  97,
    20,  26,  84,  88,  95,  95,  98,  94,
    26,  86,  91,  93,  97,  99,  98,  99,
    99, 100,  98,  99,  99,  99,  99,  99,
    99,  99,  99,  99,  99,  99,  99,  99,
    97,  97,  99,  99,  99,  99,  97,  99,
];

// Table from Relevance of Human Vision to JPEG-DCT Compression (1992) Klein, Silverstein and Carney.
// #[rustfmt::skip]
// pub const RELEVANCE_OF_HUMAN_VISION_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
//     10,  12,  14,  19,  26,  38,  57,  86,
//     12,  18,  21,  28,  35,  41,  54,  76,
//     14,  21,  25,  32,  44,  63,  92, 136,
//     19,  28,  32,  41,  54,  75, 107, 157,
//     26,  35,  44,  54,  70,  95, 132, 190,
//     38,  41,  63,  75,  95, 125, 170, 239,
//     57,  54,  92, 107, 132, 170, 227, 312,
//     86,  76, 136, 157, 190, 239, 312, 419,
// ];
//
// #[rustfmt::skip]
// pub const RELEVANCE_OF_HUMAN_VISION_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
//     10,  12,  14,  19,  26,  38,  57,  86,
//     12,  18,  21,  28,  35,  41,  54,  76,
//     14,  21,  25,  32,  44,  63,  92, 136,
//     19,  28,  32,  41,  54,  75, 107, 157,
//     26,  35,  44,  54,  70,  95, 132, 190,
//     38,  41,  63,  75,  95, 125, 170, 239,
//     57,  54,  92, 107, 132, 170, 227, 312,
//     86,  76, 136, 157, 190, 239, 312, 419,
// ];

// Table from DCTune Perceptual Optimization of Compressed Dental X-Rays (1997) Watson, Taylor, Borthwick
#[rustfmt::skip]
pub const DC_TUNE_PERCEPTUAL_OPTIMIZATION_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
      7,  8,   10,  14,  23,  44,  95, 241,
      8,  8,   11,  15,  25,  47, 102, 255,
     10,  11,  13,  19,  31,  58, 127, 255,
     14,  15,  19,  27,  44,  83, 181, 255,
     23,  25,  31,  44,  72, 136, 255, 255,
     44,  47,  58,  83, 136, 255, 255, 255,
     95, 102, 127, 181, 255, 255, 255, 255,
    241, 255, 255, 255, 255, 255, 255, 255,
];

#[rustfmt::skip]
pub const DC_TUNE_PERCEPTUAL_OPTIMIZATION_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
      7,   8,  10,  14,  23,  44,  95, 241,
      8,   8,  11,  15,  25,  47, 102, 255,
     10,  11,  13,  19,  31,  58, 127, 255,
     14,  15,  19,  27,  44,  83, 181, 255,
     23,  25,  31,  44,  72, 136, 255, 255,
     44,  47,  58,  83, 136, 255, 255, 255,
     95, 102, 127, 181, 255, 255, 255, 255,
    241, 255, 255, 255, 255, 255, 255, 255,
];

// Table from A Visual Detection Model for DCT Coefficient Quantization (1993) Ahumada, Watson, Peterson
#[rustfmt::skip]
pub const A_VISUAL_DETECTION_MODEL_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    15, 11, 11, 12, 15, 19, 25, 32,
    11, 13, 10, 10, 12, 15, 19, 24,
    11, 10, 14, 14, 16, 18, 22, 27,
    12, 10, 14, 18, 21, 24, 28, 33,
    15, 12, 16, 21, 26, 31, 36, 42,
    19, 15, 18, 24, 31, 38, 45, 53,
    25, 19, 22, 28, 36, 45, 55, 65,
    32, 24, 27, 33, 42, 53, 65, 77,
];

#[rustfmt::skip]
pub const A_VISUAL_DETECTION_MODEL_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    15, 11, 11, 12, 15, 19, 25, 32,
    11, 13, 10, 10, 12, 15, 19, 24,
    11, 10, 14, 14, 16, 18, 22, 27,
    12, 10, 14, 18, 21, 24, 28, 33,
    15, 12, 16, 21, 26, 31, 36, 42,
    19, 15, 18, 24, 31, 38, 45, 53,
    25, 19, 22, 28, 36, 45, 55, 65,
    32, 24, 27, 33, 42, 53, 65, 77,
];

// Table from An Improved Detection Model for DCT Coefficient Quantization (1993) Peterson, Ahumada and Watson
#[rustfmt::skip]
pub const AN_IMPROVED_DETECTION_MODEL_LUMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    14,  10,  11,  14,  19,  25,  34,  45,
    10,  11,  11,  12,  15,  20,  26,  33,
    11,  11,  15,  18,  21,  25,  31,  38,
    14,  12,  18,  24,  28,  33,  39,  47,
    19,  15,  21,  28,  36,  43,  51,  59,
    25,  20,  25,  33,  43,  54,  64,  74,
    34,  26,  31,  39,  51,  64,  77,  91,
    45,  33,  38,  47,  59,  74,  91, 108,
];

#[rustfmt::skip]
pub const AN_IMPROVED_DETECTION_MODEL_CHROMINANCE_QUANTIZATION_TABLE: [u8; 64] = [
    14,  10,  11,  14,  19,  25,  34,  45,
    10,  11,  11,  12,  15,  20,  26,  33,
    11,  11,  15,  18,  21,  25,  31,  38,
    14,  12,  18,  24,  28,  33,  39,  47,
    19,  15,  21,  28,  36,  43,  51,  59,
    25,  20,  25,  33,  43,  54,  64,  74,
    34,  26,  31,  39,  51,  64,  77,  91,
    45,  33,  38,  47,  59,  74,  91, 108
];

#[derive(Clone, Copy)]
pub enum QuantizationTablePreset {
    Specification,
    Flat,
    MSSIMKodakTuned,
    // ImageMagick,
    PSNRHVSNKodakTuned,
    // RelevanceOfHumanVision,
    DCTunePerceptualOptimization,
    AVisualDetectionModel,
    AnImprovedDetectionModel,
}

impl ValueEnum for QuantizationTablePreset {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Specification,
            Self::Flat,
            Self::MSSIMKodakTuned,
            // Self::ImageMagick,
            Self::PSNRHVSNKodakTuned,
            // Self::RelevanceOfHumanVision,
            Self::DCTunePerceptualOptimization,
            Self::AVisualDetectionModel,
            Self::AnImprovedDetectionModel,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let return_value = match self {
            Self::Specification => {
                PossibleValue::new("Specification").aliases(["Spec", "Default", "0"])
            }
            Self::Flat => PossibleValue::new("Flat").alias("1"),
            Self::MSSIMKodakTuned => PossibleValue::new("MSSIM-Kodak-Tuned").alias("2"),
            // Self::ImageMagick => PossibleValue::new("ImageMagick").alias("3"),
            Self::PSNRHVSNKodakTuned => PossibleValue::new("PSNR-HVS-N-Kodak-Tuned").alias("4"),
            // Self::RelevanceOfHumanVision => {
            //     PossibleValue::new("Relevance-of-human-vision").alias("5")
            // }
            Self::DCTunePerceptualOptimization => {
                PossibleValue::new("DCTune-Perceptual-Optimization").alias("6")
            }
            Self::AVisualDetectionModel => {
                PossibleValue::new("A-visual-detection-model").alias("7")
            }
            Self::AnImprovedDetectionModel => {
                PossibleValue::new("An-improved-detection-model").alias("8")
            }
        };
        Some(return_value)
    }
}

impl QuantizationTablePreset {
    pub fn to_pair(self) -> QuantizationTablePair<'static> {
        match self {
            Self::Specification => QuantizationTablePair {
                luma_table: &SPECIFICATION_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &SPECIFICATION_CHROMINANCE_QUANTIZATION_TABLE,
            },
            Self::Flat => QuantizationTablePair {
                luma_table: &FLAT_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &FLAT_CHROMINANCE_QUANTIZATION_TABLE,
            },
            Self::MSSIMKodakTuned => QuantizationTablePair {
                luma_table: &MSSIM_KODAK_TUNED_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &MSSIM_KODAK_TUNED_CHROMINANCE_QUANTIZATION_TABLE,
            },
            // Self::ImageMagick => QuantizationTablePair {
            //     luma_table: &IMAGE_MAGICK_LUMINANCE_QUANTIZATION_TABLE,
            //     chroma_table: &IMAGE_MAGICK_CHROMINANCE_QUANTIZATION_TABLE,
            // },
            Self::PSNRHVSNKodakTuned => QuantizationTablePair {
                luma_table: &PSNRHVSNI_KODAK_TUNED_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &PSNRHVSNI_KODAK_TUNED_CHROMINANCE_QUANTIZATION_TABLE,
            },
            // Self::RelevanceOfHumanVision => QuantizationTablePair {
            //     luma_table: &RELEVANCE_OF_HUMAN_VISION_LUMINANCE_QUANTIZATION_TABLE,
            //     chroma_table: &RELEVANCE_OF_HUMAN_VISION_CHROMINANCE_QUANTIZATION_TABLE,
            // },
            Self::DCTunePerceptualOptimization => QuantizationTablePair {
                luma_table: &DC_TUNE_PERCEPTUAL_OPTIMIZATION_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &DC_TUNE_PERCEPTUAL_OPTIMIZATION_CHROMINANCE_QUANTIZATION_TABLE,
            },
            Self::AVisualDetectionModel => QuantizationTablePair {
                luma_table: &A_VISUAL_DETECTION_MODEL_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &A_VISUAL_DETECTION_MODEL_CHROMINANCE_QUANTIZATION_TABLE,
            },
            Self::AnImprovedDetectionModel => QuantizationTablePair {
                luma_table: &AN_IMPROVED_DETECTION_MODEL_LUMINANCE_QUANTIZATION_TABLE,
                chroma_table: &AN_IMPROVED_DETECTION_MODEL_CHROMINANCE_QUANTIZATION_TABLE,
            },
        }
    }
}

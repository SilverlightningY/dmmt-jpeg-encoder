use crate::error::Error;
use crate::huffman::{Symbol, SymbolCodeLength};
use crate::Result;
use core::panic;
use std::fmt::Display;
use std::io;
use std::io::Write;

use super::OutputImage;
use crate::logger;

const START_OF_FILE_MARKER: [u8; 2] = [0xFF, 0xD8];
const END_OF_FILE_MARKER: [u8; 2] = [0xFF, 0xD9];
const HUFFMAN_TABLE_MARKER: [u8; 2] = [0xFF, 0xC4];
const QUANTIZATION_TABLE_MARKER: [u8; 2] = [0xFF, 0xDB];
const START_OF_FRAME_MARKER: [u8; 2] = [0xFF, 0xC0];
const START_OF_SCAN_MARKER: [u8; 2] = [0xFF, 0xDA];
const EXIF_APPLICATION_MARKER: [u8; 2] = [0xFF, 0xE1];
const JFIF_APPLICATION_MARKER: [u8; 2] = [0xFF, 0xE0];

enum ControlMarker {
    StartOfFile,
    EndOfFile,
}

enum SegmentMarker {
    HuffmanTable,
    QuantizationTable,
    ExifApplication,
    JfifApplication,
    StartOfFrame,
    StartOfScan,
}

trait AsBinaryRef {
    fn as_binary_ref(&self) -> &'static [u8];
}

impl AsBinaryRef for ControlMarker {
    fn as_binary_ref(&self) -> &'static [u8] {
        match self {
            Self::StartOfFile => &START_OF_FILE_MARKER,
            Self::EndOfFile => &END_OF_FILE_MARKER,
        }
    }
}

impl AsBinaryRef for SegmentMarker {
    fn as_binary_ref(&self) -> &'static [u8] {
        match self {
            Self::HuffmanTable => &HUFFMAN_TABLE_MARKER,
            Self::QuantizationTable => &QUANTIZATION_TABLE_MARKER,
            Self::ExifApplication => &EXIF_APPLICATION_MARKER,
            Self::JfifApplication => &JFIF_APPLICATION_MARKER,
            Self::StartOfFrame => &START_OF_FRAME_MARKER,
            Self::StartOfScan => &START_OF_SCAN_MARKER,
        }
    }
}

impl Display for SegmentMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HuffmanTable => write!(f, "Huffman Table"),
            Self::QuantizationTable => write!(f, "Quantization Table"),
            Self::ExifApplication => write!(f, "Exif Application"),
            Self::JfifApplication => write!(f, "Jfif Application"),
            Self::StartOfFrame => write!(f, "Start of Frame"),
            Self::StartOfScan => write!(f, "Start of Scan"),
        }
    }
}

#[derive(Copy, Clone)]
enum TableKind {
    LumaDC = 0b0000_0000,
    LumaAC = 0b0001_0001,
    ChromaDC = 0b0000_0010,
    ChromaAC = 0b0001_0011,
}

impl TableKind {
    fn value(&self) -> u8 {
        *self as u8
    }
}

fn create_huffman_lenght_header(code_lengths: &[SymbolCodeLength]) -> [u8; 16] {
    let mut lengths = [0; 16];
    for item in code_lengths {
        lengths[item.length - 1] += 1;
    }
    lengths
}

pub struct Encoder<'a, T> {
    writer: &'a mut T,
    image: &'a OutputImage,
}

impl<'a, T: Write> Encoder<'a, T> {
    pub fn new(writer: &'a mut T, image: &'a OutputImage) -> Encoder<'a, T> {
        Encoder { writer, image }
    }

    pub fn encode(&mut self) -> Result<()> {
        self.write_start_of_file()?;
        self.write_jfif_application_header()?;
        // self.write_luminance_quantization_table()?;
        // self.write_chrominance_quantization_table()?;
        self.write_start_of_frame()?;
        self.write_all_huffman_tables()?;
        // self.write_start_of_scan()?;
        // self.write_image_data()?;
        self.write_end_of_file()?;
        Ok(())
    }

    fn write_segment(&mut self, marker: SegmentMarker, content: &[u8]) -> io::Result<()> {
        log::info!("Writing {}", marker);
        let marker_binary_ref = marker.as_binary_ref();
        let segment_len = marker_binary_ref.len() + content.len();
        if segment_len > u16::MAX as usize {
            panic!(
                "The length of the segment '{}' is greater than u16::MAX",
                marker
            );
        }
        let segment_length = (segment_len as u16).to_be_bytes();
        logger::log_segment(marker_binary_ref, content, &segment_length);
        self.writer.write_all(marker_binary_ref)?;
        self.writer.write_all(&segment_length)?;
        self.writer.write_all(content)?;
        Ok(())
    }

    fn write_control_marker(&mut self, marker: ControlMarker) -> io::Result<()> {
        self.writer.write_all(marker.as_binary_ref())
    }

    fn write_start_of_file(&mut self) -> Result<()> {
        self.write_control_marker(ControlMarker::StartOfFile)
            .map_err(|_| Error::FailedToWriteStartOfFile)
    }

    fn write_end_of_file(&mut self) -> Result<()> {
        self.write_control_marker(ControlMarker::EndOfFile)
            .map_err(|_| Error::FailedToWriteEndOfFile)
    }

    fn write_huffman_table(
        &mut self,
        table_kind: TableKind,
        symdepths: &[SymbolCodeLength],
    ) -> Result<()> {
        let mut header: Vec<u8> = Vec::new();
        header.push(table_kind.value());
        header.extend(create_huffman_lenght_header(symdepths));
        let symbols: Vec<Symbol> = symdepths.iter().rev().map(|i| i.symbol).collect();
        header.extend(&symbols);
        self.write_segment(SegmentMarker::HuffmanTable, &header)
            .map_err(|_| Error::FailedToWriteHuffmanTables)
    }

    fn write_all_huffman_tables(&mut self) -> Result<()> {
        self.write_huffman_table(TableKind::LumaAC, &self.image.luma_ac_huffman)?;
        self.write_huffman_table(TableKind::LumaDC, &self.image.luma_dc_huffman)?;
        self.write_huffman_table(TableKind::ChromaAC, &self.image.chroma_ac_huffman)?;
        self.write_huffman_table(TableKind::ChromaDC, &self.image.chroma_dc_huffman)
    }

    fn write_jfif_application_header(&mut self) -> Result<()> {
        // let width_bytes = image.width.to_be_bytes();
        // let height_bytes = image.height.to_be_bytes();
        #[rustfmt::skip]
        let content = &[
            b'J', b'F', b'I', b'F', b'\0',// Identifier
            0x01, 0x02,             // Version
            0x00,                   // Density unit
            0x00, 0x48, 0x00, 0x48, // Density (72/0x48 common used value)
            0,                      // X Thumbnail
            0                       // Y Thumbnail
        ];
        self.write_segment(SegmentMarker::JfifApplication, content)
            .map_err(|_| Error::FailedToWriteJfifApplicationHeader)
    }

    fn write_luminance_quantization_table(&mut self) -> Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
            .map_err(|_| Error::FailedToWriteLuminanceQuantizationTable)
    }

    fn write_chrominance_quantization_table(&mut self) -> Result<()> {
        self.write_segment(SegmentMarker::QuantizationTable, &[])
            .map_err(|_| Error::FailedToWriteChrominanceQuantizationTable)
    }

    fn write_start_of_frame(&mut self) -> Result<()> {
        let width_bytes = self.image.width.to_be_bytes();
        let height_bytes = self.image.height.to_be_bytes();
        let subsampling = self.image.chroma_subsampling_preset;
        let ratio = ((4 / subsampling.horizontal_rate()) << 4) | (2 / subsampling.vertical_rate());
        #[rustfmt::skip]
        let content = &[
            self.image.bits_per_channel,                   // bits per pixel
            height_bytes[0], height_bytes[1], // image height
            width_bytes[0], width_bytes[1],   // image width
            0x03,                   // components (1 or 3)
            0x01, 0x42, 0x00,       // 0x01=y component, sampling factor, quant. table
            0x02, ratio, 0x01,       // 0x02=Cb component, ...
            0x03, ratio, 0x01,       // 0x03=Cr component, ...
        ];
        self.write_segment(SegmentMarker::StartOfFrame, content)
            .map_err(|_| Error::FailedToWriteStartOfFrame)
    }

    fn write_start_of_scan(&mut self) -> Result<()> {
        self.write_segment(SegmentMarker::StartOfScan, &[])
            .map_err(|_| Error::FailedToWriteStartOfScan)
    }

    fn write_image_data(&mut self) -> Result<()> {
        todo!("implement write image data");
    }
}

#[cfg(test)]
mod tests {
    use crate::{huffman::SymbolCodeLength, image::subsampling::ChromaSubsamplingPreset};

    use super::{super::OutputImage, Encoder, TableKind};

    const OUTPUT_IMAGE: OutputImage = OutputImage {
        width: 3,
        height: 2,
        chroma_subsampling_preset: ChromaSubsamplingPreset::P444,
        bits_per_channel: 8,
        luma_ac_huffman: Vec::new(),
        luma_dc_huffman: Vec::new(),
        chroma_ac_huffman: Vec::new(),
        chroma_dc_huffman: Vec::new(),
    };

    #[test]
    fn test_write_jfif() {
        let mut output = Vec::new();
        let image = &OUTPUT_IMAGE;
        let mut encoder = Encoder::new(&mut output, image);
        encoder.write_jfif_application_header().unwrap();
        assert_eq!(
            output,
            [
                0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', b'\0', 0x01, 0x02, 0x00, 0x00,
                0x48, 0x00, 0x48, 0, 0
            ]
        )
    }

    #[test]
    fn test_write_huffman_header() {
        let mut output = Vec::new();
        let image = &OUTPUT_IMAGE;
        let mut encoder = Encoder::new(&mut output, image);
        let symdepths =
            [(3, 2), (4, 2), (8, 4), (2, 4), (5, 4), (1, 4)].map(SymbolCodeLength::from);

        encoder
            .write_huffman_table(TableKind::LumaDC, &symdepths)
            .unwrap();

        let mut count = 0;
        while count < output.len() {
            assert_eq!(output[count], 0xFF);
            assert_eq!(output[count + 1], 0xC4);
            let skip = [output[count + 2], output[count + 3]];
            count += u16::from_le_bytes(skip) as usize;
        }
    }

    #[test]
    fn test_write_start_of_frame() {
        let mut output = Vec::new();
        let image = &OUTPUT_IMAGE;
        let mut encoder = Encoder::new(&mut output, image);
        encoder.write_start_of_frame().unwrap();

        let width_bytes = (OUTPUT_IMAGE.width).to_be_bytes();
        let height_bytes = (OUTPUT_IMAGE.height).to_be_bytes();
        let subsampling = ChromaSubsamplingPreset::P444;
        let ratio = ((4 / subsampling.horizontal_rate()) << 4) | (2 / subsampling.vertical_rate());
        assert_eq!(
            output,
            [
                0xFF,
                0xC0,
                0x00,
                0x11,
                0x08,
                height_bytes[0],
                height_bytes[1],
                width_bytes[0],
                width_bytes[1],
                0x03,
                0x01,
                0x42,
                0x00,
                0x02,
                ratio,
                0x01,
                0x03,
                ratio,
                0x01,
            ]
        )
    }
}
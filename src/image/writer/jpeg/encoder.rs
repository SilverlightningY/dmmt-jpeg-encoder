use block_fold_iterator::{BlockFoldIterator, ColorInformation};

use crate::binary_stream::BitWriter;
use crate::error::Error;
use crate::huffman::encoder::HuffmanTranslator;
use crate::huffman::{Symbol, SymbolCodeLength};
use crate::{BitPattern, Result};
use core::panic;
use std::fmt::Display;
use std::io;
use std::io::Write;

use super::segment_marker_injector::SegmentMarkerInjector;
use super::transformer::{
    categorize::CategorizedBlock, frequency_block::FrequencyBlock, quantizer::QUANTIZATION_TABLE,
};
use super::OutputImage;
use crate::logger;

mod block_fold_iterator;

const START_OF_FILE_MARKER: [u8; 2] = [0xFF, 0xD8];
const END_OF_FILE_MARKER: [u8; 2] = [0xFF, 0xD9];
const HUFFMAN_TABLE_MARKER: [u8; 2] = [0xFF, 0xC4];
const QUANTIZATION_TABLE_MARKER: [u8; 2] = [0xFF, 0xDB];
const START_OF_FRAME_MARKER: [u8; 2] = [0xFF, 0xC0];
const START_OF_SCAN_MARKER: [u8; 2] = [0xFF, 0xDA];
const JFIF_APPLICATION_MARKER: [u8; 2] = [0xFF, 0xE0];

enum ControlMarker {
    StartOfFile,
    EndOfFile,
}

enum SegmentMarker {
    HuffmanTable,
    QuantizationTable,
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
    luma_ac_huffman_translator: HuffmanTranslator,
    luma_dc_huffman_translator: HuffmanTranslator,
    chroma_ac_huffman_translator: HuffmanTranslator,
    chroma_dc_huffman_translator: HuffmanTranslator,
}

impl<'a, T: Write> Encoder<'a, T> {
    pub fn new(writer: &'a mut T, image: &'a OutputImage) -> Encoder<'a, T> {
        let luma_ac_huffman_translator = HuffmanTranslator::from(&image.luma_ac_huffman);
        let luma_dc_huffman_translator = HuffmanTranslator::from(&image.luma_dc_huffman);
        let chroma_ac_huffman_translator = HuffmanTranslator::from(&image.chroma_ac_huffman);
        let chroma_dc_huffman_translator = HuffmanTranslator::from(&image.chroma_dc_huffman);
        Encoder {
            writer,
            image,
            luma_ac_huffman_translator,
            luma_dc_huffman_translator,
            chroma_ac_huffman_translator,
            chroma_dc_huffman_translator,
        }
    }

    pub fn encode(&mut self) -> Result<()> {
        self.write_start_of_file()?;
        self.write_jfif_application_header()?;
        self.write_all_quantization_tables()?;
        self.write_start_of_frame()?;
        self.write_all_huffman_tables()?;
        self.write_start_of_scan()?;
        self.write_image_data()?;
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

    fn write_all_quantization_tables(&mut self) -> Result<()> {
        self.write_quantization_table(0)?;
        self.write_quantization_table(1)
    }

    fn write_quantization_table(&mut self, number: u8) -> Result<()> {
        let mut header: Vec<u8> = Vec::new();
        header.push(0);
        header.push(number);

        FrequencyBlock::new(QUANTIZATION_TABLE)
            .iter_zig_zag()
            .for_each(|f| header.push(*f));
        self.write_segment(SegmentMarker::QuantizationTable, &header)
            .map_err(|_| Error::FailedToWriteQuantizationTable)
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
        let data = [
            0x03, // number of components (1=mono, 3=colour)
            0x01,
            0b0001_0000, // 0x01=Y, 0x00=Huffman tables to use 0..3 ac, 0..3 dc (1 and 0)
            0x02,
            0b0011_0010, // 0x02=Cb, 0x11=Huffman tables to use 0..3 ac, 0..3 dc (3 and 2)
            0x03,
            0b0011_0010, // 0x03=Cr, 0x11=Huffman table to use 0..3 ac, 0..3 dc (3 and 2)
            // I never figured out the actual meaning of these next 3 bytes
            0x00, // start of spectral selection or predictor selection
            0x3F, // end of spectral selection
            0x00, // successive approximation bit position or point transform
        ];
        self.write_segment(SegmentMarker::StartOfScan, &data)
            .map_err(|_| Error::FailedToWriteStartOfScan)
    }

    fn write_image_data(&mut self) -> Result<()> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut segment_marker_injector = SegmentMarkerInjector::new(&mut buffer);
        let mut bit_writer = BitWriter::new(&mut segment_marker_injector, true);
        let block_fold_iterator = BlockFoldIterator::new(
            &self.image.blockwise_image_data,
            self.image.chroma_subsampling_preset,
            self.image.width as usize,
        );
        for (color_info, block) in block_fold_iterator {
            match color_info {
                ColorInformation::Luma => self.write_luma_block(&mut bit_writer, block)?,
                ColorInformation::Chroma => self.write_chroma_block(&mut bit_writer, block)?,
            }
        }
        self.writer
            .write_all(&buffer)
            .map_err(|_| Error::FailedToWriteBlock)
    }

    fn write_luma_block<W: Write>(
        &self,
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
    ) -> Result<()> {
        self.write_luma_dc_from_block(bit_writer, block)?;
        self.write_luma_ac_from_block(bit_writer, block)?;
        Ok(())
    }

    fn write_chroma_block<W: Write>(
        &self,
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
    ) -> Result<()> {
        self.write_chroma_dc_from_block(bit_writer, block)?;
        self.write_chroma_ac_from_block(bit_writer, block)?;
        Ok(())
    }

    fn write_luma_dc_from_block<W: Write>(
        &self,
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
    ) -> Result<()> {
        Self::write_dc_from_block(
            bit_writer,
            block,
            &self.luma_dc_huffman_translator,
            "luma dc",
        )
    }

    fn write_chroma_dc_from_block<W: Write>(
        &self,
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
    ) -> Result<()> {
        Self::write_dc_from_block(
            bit_writer,
            block,
            &self.chroma_dc_huffman_translator,
            "chroma dc",
        )
    }

    fn write_luma_ac_from_block<W: Write>(
        &self,
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
    ) -> Result<()> {
        Self::write_ac_from_block(
            bit_writer,
            block,
            &self.luma_ac_huffman_translator,
            "luma ac",
        )
    }

    fn write_chroma_ac_from_block<W: Write>(
        &self,
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
    ) -> Result<()> {
        Self::write_ac_from_block(
            bit_writer,
            block,
            &self.chroma_ac_huffman_translator,
            "chroma ac",
        )
    }

    fn write_dc_from_block<W: Write>(
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
        huffman_translator: &HuffmanTranslator,
        component_name: &'static str,
    ) -> Result<()> {
        let symbol = block.dc_symbol();
        let symbol = huffman_translator
            .get_code_word_for_symbol(symbol)
            .as_ref()
            .ok_or(Error::HuffmanSymbolNotPresentInTranslator(
                symbol,
                component_name,
            ))?;
        let category = block.dc_category();
        Self::write_symbol_and_category(bit_writer, symbol, category)
            .map_err(|_| Error::FailedToWriteBlock)?;
        Ok(())
    }

    fn write_symbol_and_category<W: Write>(
        bit_writer: &mut BitWriter<'_, W>,
        symbol: &impl BitPattern,
        category: &impl BitPattern,
    ) -> io::Result<()> {
        bit_writer.write_bit_pattern(symbol)?;
        bit_writer.write_bit_pattern(category)?;
        Ok(())
    }

    fn write_ac_from_block<W: Write>(
        bit_writer: &mut BitWriter<'_, W>,
        block: &CategorizedBlock,
        huffman_tranlator: &HuffmanTranslator,
        component_name: &'static str,
    ) -> Result<()> {
        for (symbol, category) in block.iter_ac_symbols().zip(block.iter_ac_categories()) {
            let symbol = huffman_tranlator
                .get_code_word_for_symbol(symbol)
                .as_ref()
                .ok_or(Error::HuffmanSymbolNotPresentInTranslator(
                    symbol,
                    component_name,
                ))?;
            Self::write_symbol_and_category(bit_writer, symbol, category)
                .map_err(|_| Error::FailedToWriteBlock)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        huffman::SymbolCodeLength,
        image::{
            subsampling::ChromaSubsamplingPreset, writer::jpeg::transformer::CombinedColorChannels,
        },
    };

    use super::{super::OutputImage, Encoder, TableKind};

    const HUFFMAN_CODES: &[SymbolCodeLength; 2] = &[
        SymbolCodeLength {
            symbol: 3,
            length: 5,
        },
        SymbolCodeLength {
            symbol: 1,
            length: 1,
        },
    ];

    fn create_test_image() -> OutputImage {
        OutputImage {
            width: 3,
            height: 2,
            chroma_subsampling_preset: ChromaSubsamplingPreset::P444,
            bits_per_channel: 8,
            luma_ac_huffman: Vec::from(HUFFMAN_CODES),
            luma_dc_huffman: Vec::from(HUFFMAN_CODES),
            chroma_ac_huffman: Vec::from(HUFFMAN_CODES),
            chroma_dc_huffman: Vec::from(HUFFMAN_CODES),
            blockwise_image_data: CombinedColorChannels {
                luma: Vec::new(),
                chroma_red: Vec::new(),
                chroma_blue: Vec::new(),
            },
        }
    }

    #[test]
    fn test_write_jfif() {
        let mut output = Vec::new();
        let image = create_test_image();
        let mut encoder = Encoder::new(&mut output, &image);
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
        let image = create_test_image();
        let mut encoder = Encoder::new(&mut output, &image);
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
        let image = create_test_image();
        let mut encoder = Encoder::new(&mut output, &image);
        encoder.write_start_of_frame().unwrap();

        let width_bytes = (image.width).to_be_bytes();
        let height_bytes = (image.height).to_be_bytes();
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
    #[test]
    fn test_write_quantization() {
        let mut output = Vec::new();
        let image = create_test_image();
        let mut encoder = Encoder::new(&mut output, &image);
        encoder.write_quantization_table(2).unwrap();

        assert_eq!(
            output,
            [
                0xFF, 0xDB, 0x00, 0x44, 0x00, 0x02, 16, 11, 12, 14, 12, 10, 16, 14, 13, 14, 18, 17,
                16, 19, 24, 40, 26, 24, 22, 22, 24, 49, 35, 37, 29, 40, 58, 51, 61, 60, 57, 51, 56,
                55, 64, 72, 92, 78, 64, 68, 87, 69, 55, 56, 80, 109, 81, 87, 95, 98, 103, 104, 103,
                62, 77, 113, 121, 112, 100, 120, 92, 101, 103, 99
            ]
        )
    }

    #[test]
    fn test_write_start_of_scan() {
        let mut output = Vec::new();
        let image = create_test_image();
        let mut encoder = Encoder::new(&mut output, &image);
        encoder.write_start_of_scan().unwrap();

        assert_eq!(
            output,
            [0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x10, 0x02, 0x32, 0x03, 0x32, 0x00, 0x3F, 0x00,]
        )
    }
}

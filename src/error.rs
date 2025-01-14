use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    PPMFileDoesNotContainRequiredToken(&'static str),
    ParsingOfTokenFailed(&'static str),
    IncompletePixelParsed(usize),
    MismatchOfSizeBetweenHeaderAndValues,
    InputFileNotFound(String),
    NoReadPermissionForInputFile(String),
    UnableToOpenInputFileForReading(String, std::io::Error),
    UnableToOpenOutputFileForWriting(String, std::io::Error),
    FailedToWriteStartOfFile,
    FailedToWriteHuffmanTables,
    FailedToWriteEndOfFile,
    FailedToWriteJfifApplicationHeader,
    FailedToWriteLuminanceQuantizationTable,
    FailedToWriteChrominanceQuantizationTable,
    FailedToWriteStartOfFrame,
    FailedToWriteStartOfScan,
    FailedToWriteImageData,
    HuffmanSymbolNotPresentInTranslator(u8, &'static str),
    FailedToWriteBlock,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PPMFileDoesNotContainRequiredToken(token_name) => {
                write!(f, "Expected token '{}' not found in PPM file", token_name)
            }
            Self::ParsingOfTokenFailed(token_name) => {
                write!(f, "Parsing of token '{}' failed", token_name)
            }
            Self::IncompletePixelParsed(number_of_tokens_parsed) => {
                write!(
                    f,
                    "Incomplete pixel parsed. Expected 3 components, but got {}.",
                    number_of_tokens_parsed
                )
            }
            Self::MismatchOfSizeBetweenHeaderAndValues => {
                write!(
                    f,
                    "Nubmer of pixels do not match the size, provided in header"
                )
            }
            Self::InputFileNotFound(path) => {
                write!(f, "Input file '{}' not found", path)
            }
            Self::NoReadPermissionForInputFile(path) => {
                write!(
                    f,
                    "Unable open file '{}' for reading. Permission denied.",
                    path
                )
            }
            Self::UnableToOpenInputFileForReading(path, error) => {
                write!(
                    f,
                    "Unable to open input file '{}' for reading: {}",
                    path, error
                )
            }
            Self::UnableToOpenOutputFileForWriting(path, error) => {
                write!(
                    f,
                    "Unable to open output file '{}' for writing: {}",
                    path, error
                )
            }
            Error::FailedToWriteStartOfFile => {
                write!(f, "Failed to write start of file control marker")
            }
            Error::FailedToWriteEndOfFile => {
                write!(f, "Failed to write end of file control marker")
            }
            Error::FailedToWriteHuffmanTables => {
                write!(f, "Failed to write huffmann tables")
            }
            Error::FailedToWriteJfifApplicationHeader => {
                write!(f, "Failed to write JFIF application header")
            }
            Error::FailedToWriteLuminanceQuantizationTable => {
                write!(f, "Failed to write luminance quantization table")
            }
            Error::FailedToWriteChrominanceQuantizationTable => {
                write!(f, "Failed to write chrominance quantization table")
            }
            Error::FailedToWriteStartOfFrame => write!(f, "Failed to write start of frame segment"),
            Error::FailedToWriteStartOfScan => write!(f, "Failed to write start of scan segment"),
            Error::FailedToWriteImageData => write!(f, "Failed to write image data"),
            Error::HuffmanSymbolNotPresentInTranslator(symbol, translator) => {
                write!(
                    f,
                    "Huffman symbol '{}' not present in {} translator",
                    symbol, translator
                )
            }
            Error::FailedToWriteBlock => write!(f, "Failed to write image block"),
        }
    }
}

impl std::error::Error for Error {}

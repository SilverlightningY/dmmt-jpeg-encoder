use crate::image::subsampling::ChromaSubsamplingPreset;
use crate::image::writer::jpeg::QuantizationTablePreset;
use crate::Arguments;
use clap::{
    arg, builder::PossibleValue, crate_authors, crate_description, crate_name, crate_version,
    value_parser, Arg, ArgMatches, Command,
};
use std::ffi::OsString;
use std::path::PathBuf;
use std::{io, thread};

pub struct CLIParser {
    command: Command,
}

impl CLIParser {
    pub fn new() -> Self {
        let command = Self::create_base_command();
        let command = Self::register_arguments(command);
        CLIParser { command }
    }

    pub fn parse<I, T>(&mut self, itr: I) -> Arguments
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let matches = self
            .command
            .try_get_matches_from_mut(itr)
            .unwrap_or_else(|e| e.exit());
        Self::extract_arguments(&matches)
    }

    fn register_arguments(command: Command) -> Command {
        let command = Self::register_input_file_argument(command);
        let command = Self::register_output_file_argument(command);
        let command = Self::register_bits_per_channel_argument(command);
        let command = Self::register_chroma_subsampling_preset_argument(command);
        let command = Self::register_threads_argument(command);
        Self::register_quantization_table_preset_argument(command)
    }

    fn register_input_file_argument(command: Command) -> Command {
        command.arg(Self::create_input_file_argument())
    }

    fn register_output_file_argument(command: Command) -> Command {
        command.arg(Self::create_output_file_argument())
    }

    fn register_bits_per_channel_argument(command: Command) -> Command {
        command.arg(Self::create_bits_per_channel_argument())
    }

    fn register_chroma_subsampling_preset_argument(command: Command) -> Command {
        command.arg(Self::create_chroma_subsampling_preset_argument())
    }

    fn register_threads_argument(command: Command) -> Command {
        command.arg(Self::create_threads_argument())
    }

    fn register_quantization_table_preset_argument(command: Command) -> Command {
        command.arg(Self::create_quantization_table_preset_argument())
    }

    fn create_base_command() -> Command {
        Command::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
    }

    fn create_input_file_argument() -> Arg {
        Arg::new("input_file")
            .help("Path to PPM imput file")
            .value_parser(value_parser!(PathBuf))
            .required(true)
    }

    fn create_output_file_argument() -> Arg {
        Arg::new("output_file")
            .help("Path to JPEG output file")
            .value_parser(value_parser!(PathBuf))
            .required(true)
    }

    fn create_bits_per_channel_argument() -> Arg {
        arg!(bits_per_channel: -b --bits_per_channel <BITS> "Bits per color channel")
            .default_value("8")
            .value_parser([
                PossibleValue::new("8"),
                PossibleValue::new("16"),
                PossibleValue::new("32"),
            ])
    }

    fn create_chroma_subsampling_preset_argument() -> Arg {
        arg!(chroma_subsampling_preset: -p --chroma_subsampling_preset <PRESET> "Chroma subsampling preset")
            .default_value("P420").value_parser(value_parser!(ChromaSubsamplingPreset))
    }

    fn create_threads_argument() -> Arg {
        arg!(-t --threads <THREADS> "Number of Threads")
            .default_value(get_number_of_threads().unwrap_or(1).to_string())
            .required(false)
            .value_parser(value_parser!(usize))
    }

    fn create_quantization_table_preset_argument() -> Arg {
        arg!(quantization_table_preset: -q --quantization_table <TABLE> "Quantization table preset")
            .default_value("Specification")
            .value_parser(value_parser!(QuantizationTablePreset))
    }

    fn extract_arguments(matches: &ArgMatches) -> Arguments {
        Arguments {
            input_file: Self::extract_input_file_argument(matches),
            output_file: Self::extract_output_file_argument(matches),
            chroma_subsampling_preset: Self::extract_chroma_subsampling_preset_argument(matches),
            bits_per_channel: Self::extract_bits_per_channel_argument(matches),
            number_of_threads: Self::extract_threads_argument(matches),
            quantization_table_preset: Self::extract_quantization_table_preset_argument(matches),
        }
    }

    fn extract_input_file_argument(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("input_file")
            .expect("Required argument input_file not provided")
            .clone()
    }

    fn extract_output_file_argument(matches: &ArgMatches) -> PathBuf {
        matches
            .get_one::<PathBuf>("output_file")
            .expect("Required argument output_file not provided")
            .clone()
    }

    fn extract_bits_per_channel_argument(matches: &ArgMatches) -> u8 {
        matches
            .get_one::<String>("bits_per_channel")
            .expect("Bits per channel must be provided, but was unset.")
            .parse::<u8>()
            .expect("Argument value for bits per channel must be in range of u8")
    }

    fn extract_chroma_subsampling_preset_argument(matches: &ArgMatches) -> ChromaSubsamplingPreset {
        matches
            .get_one::<ChromaSubsamplingPreset>("chroma_subsampling_preset")
            .expect("Chroma subsampling preset must be provided, but was unset.")
            .to_owned()
    }

    fn extract_threads_argument(matches: &ArgMatches) -> usize {
        matches
            .get_one::<usize>("threads")
            .expect("Required argument threads not provided")
            .to_owned()
    }

    fn extract_quantization_table_preset_argument(matches: &ArgMatches) -> QuantizationTablePreset {
        matches
            .get_one::<QuantizationTablePreset>("quantization_table_preset")
            .expect("Quantization table preset must be provided, but was unset")
            .to_owned()
    }
}

impl Default for CLIParser {
    fn default() -> Self {
        Self::new()
    }
}

fn get_number_of_threads() -> io::Result<usize> {
    Ok(thread::available_parallelism()?.get())
}

#[cfg(test)]
mod tests {
    use clap::{error::ErrorKind, Command};

    use super::{CLIParser, ChromaSubsamplingPreset};

    const PROGRAM_NAME_ARGUMENT: &str = "test_program_name";

    #[test]
    fn parse_input_file_argument() {
        let input_file_name = "testfile.ppm";
        let command = Command::new("test");
        let command = CLIParser::register_input_file_argument(command);
        let matches = command.get_matches_from(vec![PROGRAM_NAME_ARGUMENT, input_file_name]);
        let input_file = CLIParser::extract_input_file_argument(&matches);
        assert_eq!(input_file.file_name().unwrap(), input_file_name);
    }

    #[test]
    fn parse_output_file_argument() {
        let output_file_name = "testfile.ppm";
        let command = Command::new("test");
        let command = CLIParser::register_output_file_argument(command);
        let matches = command.get_matches_from(vec![PROGRAM_NAME_ARGUMENT, output_file_name]);
        let output_file = CLIParser::extract_output_file_argument(&matches);
        assert_eq!(output_file.file_name().unwrap(), output_file_name);
    }

    #[test]
    fn parse_bits_per_channel_argument() {
        let expected_bits_per_channel = 16;
        let command = Command::new("test");
        let command = CLIParser::register_bits_per_channel_argument(command);
        let matches =
            command.get_matches_from(vec![PROGRAM_NAME_ARGUMENT, "--bits_per_channel", "16"]);
        let bits_per_channel = CLIParser::extract_bits_per_channel_argument(&matches);
        assert_eq!(bits_per_channel, expected_bits_per_channel);
    }

    #[test]
    fn parse_bits_per_channel_illegal_argument() {
        let command = Command::new("test");
        let command = CLIParser::register_bits_per_channel_argument(command);
        let result =
            command.try_get_matches_from(vec![PROGRAM_NAME_ARGUMENT, "--bits_per_channel", "11"]);
        if let Err(error) = result {
            assert_eq!(error.kind(), ErrorKind::InvalidValue);
        } else {
            panic!("Illegal value for bits_per_channel not detected");
        }
    }

    #[test]
    fn parse_chroma_subsampling_preset_argument() {
        let command = Command::new("test");
        let command = CLIParser::register_chroma_subsampling_preset_argument(command);
        let matches = command.get_matches_from(vec![
            PROGRAM_NAME_ARGUMENT,
            "--chroma_subsampling_preset",
            "P444",
        ]);
        let actual_preset = CLIParser::extract_chroma_subsampling_preset_argument(&matches);
        let expected_preset = ChromaSubsamplingPreset::P444;
        assert_eq!(actual_preset, expected_preset);
    }

    #[test]
    fn parse_number_of_threads_argument() {
        let command = Command::new("test");
        let command = CLIParser::register_threads_argument(command);
        let matches = command.get_matches_from(vec![PROGRAM_NAME_ARGUMENT, "--threads", "5"]);
        let actual = CLIParser::extract_threads_argument(&matches);
        let expected = 5;
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_required_arguments_only() {
        let input_file_name = "inputfile.ppm";
        let input_file_path = format!("/input_directory/{}", input_file_name);
        let output_file_name = "outputfile.jpg";
        let output_file_path = format!("/output_directory/{}", output_file_name);
        let mut cli_parser = CLIParser::default();
        let arguments = cli_parser.parse(vec![
            PROGRAM_NAME_ARGUMENT,
            &input_file_path,
            &output_file_path,
            "-t",
            "8",
        ]);
        assert_eq!(
            arguments.input_file.file_name().unwrap(),
            input_file_name,
            "input file does not match"
        );
        assert_eq!(
            arguments.output_file.file_name().unwrap(),
            output_file_name,
            "output file does not match"
        );
        assert_eq!(
            arguments.bits_per_channel, 8,
            "bits_per_channel does not match"
        );
        assert_eq!(
            arguments.chroma_subsampling_preset,
            ChromaSubsamplingPreset::P420,
            "chroma_subsampling_preset does not match"
        );
        assert_eq!(
            arguments.number_of_threads, 8,
            "number_of_threads does not match"
        );
    }
}

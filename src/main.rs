use std::path::PathBuf;

use clap::{
    arg, crate_authors, crate_description, crate_name, crate_version, value_parser, Command,
};

struct Arguments {
    input_file: PathBuf,
    output_file: PathBuf,
    quality: u8,
}

fn parse_cli_arguments() -> Arguments {
    let arguments = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            arg!(input_file: -i --input_file <FILE> "Source image in PPM P3 format")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(output_file: -o --output_file <FILE> "Target JPEG file location")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(quality: -q --quality <INT> "Quality of JPEG compression from 0 to 100")
                .required(false)
                .value_parser(value_parser!(u8).range(0..100))
                .default_value("90"),
        )
        .get_matches();
    Arguments {
        input_file: arguments
            .get_one::<PathBuf>("input_file")
            .expect("required argument input_file not provided")
            .clone(),
        output_file: arguments
            .get_one::<PathBuf>("output_file")
            .expect("required argument output_file not provided")
            .clone(),
        quality: *arguments
            .get_one::<u8>("quality")
            .expect("argument quality not provided"),
    }
}

fn main() {
    let arguments = parse_cli_arguments();
    println!(
        "Input file: {}; Output file: {}; Quality: {}",
        arguments.input_file.display(),
        arguments.output_file.display(),
        arguments.quality
    );
}

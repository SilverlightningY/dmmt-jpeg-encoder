use std::env::args_os;

use dmmt_jpeg_encoder::{convert_ppm_to_jpeg, CLIParser};

fn main() {
    let mut cli_parser = CLIParser::default();
    let arguments = cli_parser.parse(args_os());
    match convert_ppm_to_jpeg(&arguments) {
        Ok(_) => println!("Conversion successful"),
        Err(e) => eprintln!("Conversion failed because of: {}", e),
    }
}

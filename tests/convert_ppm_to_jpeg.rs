use dmmt_jpeg_encoder::{convert_ppm_to_jpeg, CLIParser};
use std::path::PathBuf;
use std::{env, fs};

const INPUT_IMAGE_PATH: &str = "tests/image.ppm";
const RESULT_IMAGE_PATH: &str = "tests/result.jpg";

fn get_project_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn get_input_image_path() -> PathBuf {
    let mut root_path = get_project_root_path();
    root_path.push(INPUT_IMAGE_PATH);
    root_path
}

fn get_result_image_path() -> PathBuf {
    let mut root_path = get_project_root_path();
    root_path.push(RESULT_IMAGE_PATH);
    root_path
}

fn cleanup() {
    let result_image_path = get_result_image_path();
    if result_image_path.exists() && result_image_path.is_file() {
        fs::remove_file(result_image_path).expect("Deletion of output file failed");
    }
}

#[test]
fn test_convert_ppm_to_jpeg() {
    cleanup();
    let result_image_path = get_result_image_path();
    let mut cli_parser = CLIParser::new();
    let arguments = cli_parser.parse(vec![
        "test",
        get_input_image_path().to_str().unwrap(),
        result_image_path.to_str().unwrap(),
    ]);
    convert_ppm_to_jpeg(&arguments).expect("Conversion failed");
    assert!(result_image_path.exists(), "Output file was not created");
}

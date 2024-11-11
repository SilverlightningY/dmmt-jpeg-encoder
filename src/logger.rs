#[ctor::ctor]
fn init() {
    use log4rs;
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
}

pub fn log_segment(marker: &[u8], content: &[u8], segment_length: &[u8]) {
    fn get_byte_array(bytes: &[u8]) -> Vec<String> {
        bytes.iter().map(|byte| format!("{:02X}", byte)).collect()
    }
    log::info!(
        "{:?} {:?}\n{:?}",
        get_byte_array(marker),
        get_byte_array(segment_length),
        get_byte_array(content)
    );
}

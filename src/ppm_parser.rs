use std::fs;
use std::io::{self, BufRead};

use crate::color::{RGBColorFormat, RangeColorFormat, YCbCrColorFormat};
use crate::image;

pub fn read_ppm(file_path: &str) -> Result<image::Image<f32>, String> {
    let mut header: Vec<String> = Vec::new();

    let mut luma: Vec<f32> = Vec::new();
    let mut chroma_blue: Vec<f32> = Vec::new();
    let mut chroma_red: Vec<f32> = Vec::new();

    let file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();

    while header.len() < 4 {
        if let Some(line_result) = lines.next() {
            match line_result {
                Ok(content) => {
                    let elements: Vec<String> = content
                        .split('#')
                        .next()
                        .unwrap_or(&content)
                        .split_whitespace()
                        .map(|v| v.to_string())
                        .collect();

                    header.extend(elements);
                }
                Err(e) => {
                    eprintln!("Error reading line: {}", e);
                }
            }
        } else {
            break;
        }
    }

    if header[0] != "P3" {
        return Err("Image File does not start with 'P3'".to_string());
    }
    let max: u16 = header[3].clone().parse().unwrap();

    println!("{:?}", header);

    let mut pixel: Vec<u16> = Vec::new();

    for line in lines {
        let content = line.map_err(|e| e.to_string())?;

        if content.starts_with('#') {
            continue;
        }

        for value in content.split_whitespace() {
            pixel.push(value.parse().unwrap());
            if pixel.len() == 3 {
                let col: RangeColorFormat<u16> =
                    RangeColorFormat::new(max, pixel[0], pixel[1], pixel[2]);
                let rgb = RGBColorFormat::from(&col);
                let result: YCbCrColorFormat<f32> = YCbCrColorFormat::from(&rgb);
                luma.push(result.luma);
                chroma_blue.push(result.chroma_blue);
                chroma_red.push(result.chroma_red);

                pixel.clear();
            }
        }
    }

    Ok(image::Image::<f32> {
        width: header[1].parse().unwrap(),
        height: header[2].parse().unwrap(),
        luma: luma.to_vec(),
        chroma_blue: chroma_blue.to_vec(),
        chroma_red: chroma_red.to_vec(),
    })
}

#[cfg(test)]
mod test {
    use crate::image::Image;
    use crate::ppm_parser;

    #[test]
    fn read_image() {
        let image: Image<f32> = ppm_parser::read_ppm("src/image2.ppm").expect("abc");
        assert!(image.height == 480)
    }
}

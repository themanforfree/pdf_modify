use std::{fs::File, io::Read, path::Path};

use anyhow::Result;
use image::GenericImageView;
use lopdf::{Stream, dictionary};

pub(crate) fn load_and_split_png(path: impl AsRef<Path>) -> Result<(Stream, Stream, u32, u32)> {
    let mut buffer = Vec::new();
    let mut file = File::open(path)?;
    file.read_to_end(&mut buffer)?;
    let img = image::load_from_memory(&buffer)?;
    assert!(img.color() == image::ColorType::Rgba8); // Ensure the image is RGBA
    let (width, height) = img.dimensions();
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    let mut alpha_data = Vec::with_capacity((width * height) as usize);
    for (_x, _y, pixel) in img.pixels() {
        rgb_data.extend_from_slice(&pixel.0[0..3]); // R,G,B
        alpha_data.push(pixel.0[3]); // A
    }
    let mut alpha_stream = Stream::new(
        dictionary!(
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => width,
            "Height" => height,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        ),
        alpha_data,
    );
    alpha_stream.compress()?;
    let mut rgb_stream = Stream::new(
        dictionary!(
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => width,
            "Height" => height,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
        ),
        rgb_data,
    );
    rgb_stream.compress()?;
    Ok((rgb_stream, alpha_stream, width, height))
}

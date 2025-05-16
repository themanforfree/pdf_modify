use std::path::Path;

use anyhow::Result;
use image::{GenericImageView, GrayImage, RgbImage, imageops};
use lopdf::{Stream, dictionary};

pub(crate) struct ImageHelper {
    rgb_image: RgbImage,
    alpha_image: GrayImage,
    divider: Divider,
}

impl ImageHelper {
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_and_split(path, 1)
    }

    pub(crate) fn load_and_split(path: impl AsRef<Path>, parts: usize) -> Result<Self> {
        let raw_image = image::open(path)?;
        if raw_image.color() != image::ColorType::Rgba8 {
            return Err(anyhow::anyhow!("Only RGBA images are supported"));
        }
        let (width, height) = raw_image.dimensions();
        let mut rgb_image = RgbImage::new(width, height);
        let mut alpha_image = GrayImage::new(width, height);
        for ((rgb, alpha), (_, _, rgba)) in rgb_image
            .pixels_mut()
            .zip(alpha_image.pixels_mut())
            .zip(raw_image.pixels())
        {
            rgb[0] = rgba[0];
            rgb[1] = rgba[1];
            rgb[2] = rgba[2];
            alpha[0] = rgba[3];
        }
        let divider = Divider::new(rgb_image.width() as usize, parts);
        Ok(ImageHelper {
            rgb_image,
            alpha_image,
            divider,
        })
    }

    fn img_data_to_stream(
        data: Vec<u8>,
        width: u32,
        height: u32,
        color_space: &str,
    ) -> Result<Stream> {
        let mut stream = Stream::new(
            dictionary!(
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => width,
                "Height" => height,
                "ColorSpace" => color_space,
                "BitsPerComponent" => 8,
            ),
            data,
        );
        stream.compress()?;
        Ok(stream)
    }

    /// Get the splitted RGB image stream for the given page number.
    pub(crate) fn get_img_pair(&mut self, page_number: Option<usize>) -> Result<(Stream, Stream)> {
        let (rgb_data, alpha_data, width, height) = match page_number {
            Some(pn) => {
                let height = self.rgb_image.height();
                let (x, width) = self
                    .divider
                    .get(pn)
                    .ok_or_else(|| anyhow::anyhow!("Invalid page number"))?;
                let rgb_data = imageops::crop(&mut self.rgb_image, x, 0, width, height)
                    .to_image()
                    .into_vec();
                let alpha_data = imageops::crop(&mut self.alpha_image, x, 0, width, height)
                    .to_image()
                    .into_vec();
                (rgb_data, alpha_data, width, height)
            }
            None => {
                let (width, height) = self.rgb_image.dimensions();
                let rgb_data = self.rgb_image.to_vec();
                let alpha_data = self.alpha_image.to_vec();
                (rgb_data, alpha_data, width, height)
            }
        };
        let rgb = Self::img_data_to_stream(rgb_data, width, height, "DeviceRGB")?;
        let alpha = Self::img_data_to_stream(alpha_data, width, height, "DeviceGray")?;
        Ok((rgb, alpha))
    }
}

#[derive(Debug, Default)]
struct Divider {
    parts: usize,
    base: usize,
    remainder: usize,
}

impl Divider {
    fn new(total: usize, parts: usize) -> Self {
        let base = if parts == 0 { 0 } else { total / parts };
        let remainder = if parts == 0 { 0 } else { total % parts };
        Self {
            parts,
            base,
            remainder,
        }
    }

    fn get(&self, index: usize) -> Option<(u32, u32)> {
        if index >= self.parts {
            return None;
        }
        let start = self.base * index + index.min(self.remainder);
        let width = if index < self.remainder {
            self.base + 1
        } else {
            self.base
        };
        Some((start as u32, width as u32))
    }
}

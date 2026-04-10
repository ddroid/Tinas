pub mod resizer;
pub mod optimizer;

use image::{DynamicImage, ImageFormat as ImgFormat};

pub fn detect_format(data: &[u8]) -> super::pak::ImageFormat {
    use super::pak::ImageFormat;
    
    if data.len() >= 8 {
        // PNG magic bytes
        if &data[0..8] == &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return ImageFormat::Png;
        }
        // WebP magic bytes: RIFF....WEBP
        if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
            return ImageFormat::Webp;
        }
    }
    ImageFormat::Unknown
}

pub fn resize_to_fit(image: &DynamicImage, target_width: u32, target_height: u32) -> DynamicImage {
    image.resize_exact(target_width, target_height, image::imageops::Lanczos3)
}

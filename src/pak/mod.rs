pub mod parser;
pub mod writer;

use anyhow::{Result, Context};
use image::DynamicImage;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFormat {
    Png,
    Webp,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub id: u16,
    pub offset: u32,
    pub data: Vec<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<ImageFormat>,
}

impl Resource {
    pub fn detect_format(&mut self) {
        self.format = Some(detect_format(&self.data));
    }

    pub fn parse_image_dimensions(&mut self) -> Result<()> {
        self.detect_format();
        match self.format {
            Some(ImageFormat::Png) => {
                if let Some((w, h)) = parse_png_dimensions(&self.data) {
                    self.width = Some(w);
                    self.height = Some(h);
                }
            }
            Some(ImageFormat::Webp) => {
                if let Some((w, h)) = parse_webp_dimensions(&self.data) {
                    self.width = Some(w);
                    self.height = Some(h);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn is_image(&self) -> bool {
        matches!(self.format, Some(ImageFormat::Png) | Some(ImageFormat::Webp))
    }
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub id: u16,
    pub resource_id: u16,
    pub encoding: u8,
}

#[derive(Debug, Clone)]
pub struct PakFile {
    pub version: u32,
    pub encoding: u8,
    pub resources: Vec<Resource>,
    pub aliases: Vec<Alias>,
}

impl PakFile {
    pub fn load(path: &Path) -> Result<Self> {
        parser::parse_pak_file(path)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        writer::write_pak_file(self, path)
    }

    pub fn get_image(&self, id: u16) -> Option<DynamicImage> {
        let resource = self.resources.iter().find(|r| r.id == id)?;
        decode_image(&resource.data).ok()
    }

    pub fn replace_resource(&mut self, id: u16, data: Vec<u8>) -> Result<()> {
        let resource = self.resources
            .iter_mut()
            .find(|r| r.id == id)
            .context("Resource not found")?;
        
        resource.data = data;
        resource.parse_image_dimensions()?;
        Ok(())
    }

    pub fn get_image_resources(&self) -> Vec<&Resource> {
        self.resources.iter().filter(|r| r.is_image()).collect()
    }
}

fn detect_format(data: &[u8]) -> ImageFormat {
    if data.len() >= 8 {
        // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
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

fn parse_png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 {
        return None;
    }
    // IHDR chunk starts at byte 16
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((width, height))
}

fn parse_webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 30 {
        return None;
    }
    
    // VP8 chunk
    if &data[12..16] == b"VP8 " {
        // VP8 simple format
        let b0 = data[26];
        let b1 = data[27];
        let b2 = data[28];
        let b3 = data[29];
        
        let width = ((b1 & 0x3F) as u32) << 8 | (b0 as u32);
        let height = ((b3 & 0x3F) as u32) << 8 | (b2 as u32);
        return Some((width, height));
    }
    
    // VP8X chunk
    if &data[12..16] == b"VP8X" {
        if data.len() < 28 {
            return None;
        }
        let width = 1 + u32::from_le_bytes([0, data[24], data[25], data[26]]);
        let height = 1 + u32::from_le_bytes([0, data[27], data[28], data[29]]);
        return Some((width, height));
    }
    
    None
}

fn decode_image(data: &[u8]) -> Result<DynamicImage> {
    let format = detect_format(data);
    match format {
        ImageFormat::Png | ImageFormat::Webp => {
            let img = image::load_from_memory(data)?;
            Ok(img)
        }
        _ => anyhow::bail!("Unsupported image format"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn roundtrip_brave_pak() {
        let input = Path::new("/tmp/brave_original.pak");
        if !input.exists() {
            eprintln!("Skipping roundtrip test: /tmp/brave_original.pak not found");
            return;
        }
        let output = Path::new("/tmp/brave_roundtrip.pak");

        let pak = PakFile::load(input).expect("Failed to load");
        eprintln!("Loaded: {} resources, {} aliases, version={}", pak.resources.len(), pak.aliases.len(), pak.version);

        pak.save(output).expect("Failed to save");

        let size_in = std::fs::metadata(input).unwrap().len();
        let size_out = std::fs::metadata(output).unwrap().len();
        eprintln!("Input size:  {}", size_in);
        eprintln!("Output size: {}", size_out);

        assert_eq!(size_in, size_out, "Roundtrip file size mismatch");

        // Compare bytes
        let bytes_in = std::fs::read(input).unwrap();
        let bytes_out = std::fs::read(output).unwrap();
        if bytes_in != bytes_out {
            // Find first difference
            for (i, (a, b)) in bytes_in.iter().zip(bytes_out.iter()).enumerate() {
                if a != b {
                    panic!("First byte difference at offset {}: original=0x{:02x} written=0x{:02x}", i, a, b);
                }
            }
        }
    }
}

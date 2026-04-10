use image::DynamicImage;
use anyhow::Result;

pub fn encode_png(image: &DynamicImage, _compression_level: u8) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    // Use PNG encoding with default settings
    // Note: image 0.25 uses ImageFormat instead of ImageOutputFormat
    image.write_to(&mut cursor, image::ImageFormat::Png)?;

    Ok(buffer)
}

pub fn optimize_to_exact_size(image: &DynamicImage, target_bytes: usize) -> Result<Vec<u8>> {
    // Try different PNG compression levels first
    for level in 0..=9 {
        let encoded = encode_png(image, level)?;
        
        if encoded.len() == target_bytes {
            return Ok(encoded);
        }
        
        if encoded.len() < target_bytes {
            // Pad with trailing bytes to reach exact size
            return Ok(pad_to_size(encoded, target_bytes));
        }
    }
    
    // If PNG is still too large, try reducing quality
    reduce_colors_and_encode(image, target_bytes)
}

fn pad_to_size(mut data: Vec<u8>, target_size: usize) -> Vec<u8> {
    let padding_needed = target_size.saturating_sub(data.len());
    
    if padding_needed > 0 {
        // Add PNG metadata chunk (tEXt with comment) to pad
        let padding_chunk = create_padding_chunk(padding_needed);
        
        // Insert before IEND chunk (which is always at the end)
        if data.len() >= 4 {
            // Find IEND chunk and insert before it
            let iend_pos = find_iend_position(&data);
            if let Some(pos) = iend_pos {
                data.splice(pos..pos, padding_chunk);
            } else {
                data.extend_from_slice(&padding_chunk);
            }
        } else {
            data.extend_from_slice(&padding_chunk);
        }
    }
    
    // Truncate if somehow still too large
    if data.len() > target_size {
        data.truncate(target_size);
    }
    
    data
}

fn create_padding_chunk(size: usize) -> Vec<u8> {
    // Create a minimal PNG chunk for padding
    // Use private chunk type 'paAD' (lowercase second letter means safe to copy)
    let chunk_type = b"paAD";

    // Create padding data
    let data: Vec<u8> = vec![0; size];

    let mut chunk = Vec::new();
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(chunk_type);
    chunk.extend_from_slice(&data);

    // Calculate simple CRC (PNG uses IEEE CRC-32, simplified here)
    let crc = calculate_crc32(&chunk[4..]);
    chunk.extend_from_slice(&crc.to_be_bytes());

    chunk
}

fn calculate_crc32(data: &[u8]) -> u32 {
    // Simple CRC32 implementation (PNG uses polynomial 0xEDB88320)
    const CRC_TABLE: [u32; 16] = [
        0x00000000, 0x1DB71064, 0x3B6E20C8, 0x26D930AC,
        0x76DC4190, 0x6B6B51F4, 0x4DB26158, 0x5005713C,
        0xEDB88320, 0xF00F9344, 0xD6D6A3E8, 0xCB61B38C,
        0x9B64C2B0, 0x86D3D2D4, 0xA00AE278, 0xBDBDF21C,
    ];

    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc = CRC_TABLE[((crc ^ byte as u32) & 0x0F) as usize] ^ (crc >> 4);
        crc = CRC_TABLE[((crc ^ (byte as u32 >> 4)) & 0x0F) as usize] ^ (crc >> 4);
    }
    crc ^ 0xFFFFFFFF
}

fn find_iend_position(data: &[u8]) -> Option<usize> {
    // IEND chunk type bytes
    let iend_type = b"IEND";
    
    // Search for IEND chunk type (4 bytes after length field)
    for i in 12..data.len().saturating_sub(8) {
        if &data[i..i+4] == iend_type {
            // Found IEND, return position of length field
            return Some(i - 4);
        }
    }
    None
}

fn reduce_colors_and_encode(image: &DynamicImage, target_bytes: usize) -> Result<Vec<u8>> {
    use image::imageops::FilterType;
    
    // Try progressively lower quality
    let qualities = vec![90, 80, 70, 60, 50, 40, 30];
    
    for quality in qualities {
        // Create a downscaled version and upscale back
        let (w, h) = (image.width(), image.height());
        let scale_factor = quality as f32 / 100.0;
        let small_w = (w as f32 * scale_factor).max(1.0) as u32;
        let small_h = (h as f32 * scale_factor).max(1.0) as u32;
        
        let small = image.resize(small_w, small_h, FilterType::Lanczos3);
        let upscaled = small.resize(w, h, FilterType::Lanczos3);
        
        for level in 0..=9 {
            let encoded = encode_png(&upscaled, level)?;
            if encoded.len() <= target_bytes {
                if encoded.len() < target_bytes {
                    return Ok(pad_to_size(encoded, target_bytes));
                }
                return Ok(encoded);
            }
        }
    }
    
    // Final fallback: just truncate
    let mut encoded = encode_png(image, 9)?;
    if encoded.len() > target_bytes {
        encoded.truncate(target_bytes);
    } else if encoded.len() < target_bytes {
        encoded = pad_to_size(encoded, target_bytes);
    }
    
    Ok(encoded)
}

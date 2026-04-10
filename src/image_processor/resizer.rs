use image::DynamicImage;

pub fn resize_to_exact_fit(image: &DynamicImage, target_width: u32, target_height: u32) -> DynamicImage {
    image.resize_exact(target_width, target_height, image::imageops::Lanczos3)
}

pub fn center_crop_or_pad(image: &DynamicImage, target_width: u32, target_height: u32) -> DynamicImage {
    let (width, height) = (image.width(), image.height());
    
    if width == target_width && height == target_height {
        return image.clone();
    }
    
    // Create a new image with target size
    let mut output = DynamicImage::new_rgba8(target_width, target_height);
    
    // Calculate centering offsets
    let x_offset = if target_width > width {
        (target_width - width) / 2
    } else {
        0
    };
    let y_offset = if target_height > height {
        (target_height - height) / 2
    } else {
        0
    };
    
    // If target is larger, paste original in center
    if target_width >= width && target_height >= height {
        image::imageops::overlay(&mut output, image, x_offset as i64, y_offset as i64);
    } else {
        // Crop to fit
        let crop_x = if width > target_width {
            (width - target_width) / 2
        } else {
            0
        };
        let crop_y = if height > target_height {
            (height - target_height) / 2
        } else {
            0
        };
        
        let cropped = image.crop_imm(crop_x, crop_y, 
            target_width.min(width), 
            target_height.min(height));
        
        image::imageops::overlay(&mut output, &cropped, 0, 0);
    }
    
    output
}

pub fn resize_to_fit(image: &DynamicImage, target_width: u32, target_height: u32) -> DynamicImage {
    // Use resize to fit within bounds while maintaining aspect ratio
    image.resize(target_width, target_height, image::imageops::Lanczos3)
}

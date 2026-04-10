use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Result, Context};
use byteorder::{WriteBytesExt, LittleEndian};

use super::PakFile;

pub fn write_pak_file(pak: &PakFile, path: &Path) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create pak file: {}", path.display()))?;
    
    let resource_count = pak.resources.len();
    let alias_count = pak.aliases.len();
    
    // Calculate header size
    let header_size = if pak.version == 4 {
        16 + resource_count * 6 // v4: 2 byte id + 4 byte offset
    } else {
        16 + resource_count * 8 // v5: 2 byte id + 4 byte offset + 2 byte flags
    };
    
    let aliases_offset = header_size;
    let data_start = aliases_offset + alias_count * 8; // aliases: 2 + 2 + 4 padding
    
    // Write header based on version
    if pak.version == 4 {
        file.write_all(b"PK\x03\x04")?; // v4 magic
    } else {
        file.write_all(b"PK\x05\x06")?; // v5 magic
    }
    
    // Version and other header fields
    file.write_u16::<LittleEndian>(pak.version as u16)?;
    file.write_u8(0)?; // Unused
    file.write_u8(pak.encoding)?;
    file.write_u32::<LittleEndian>(resource_count as u32)?;
    file.write_u32::<LittleEndian>(alias_count as u32)?;
    
    // Build resource data and calculate offsets
    let mut resource_data: Vec<u8> = Vec::new();
    let mut resource_offsets = Vec::new();
    
    for resource in &pak.resources {
        resource_offsets.push(data_start + resource_data.len());
        resource_data.extend_from_slice(&resource.data);
        
        // Ensure 4-byte alignment
        let padding = (4 - (resource.data.len() % 4)) % 4;
        for _ in 0..padding {
            resource_data.push(0);
        }
    }
    
    // Write resource table
    for (i, resource) in pak.resources.iter().enumerate() {
        file.write_u16::<LittleEndian>(resource.id)?;
        file.write_u32::<LittleEndian>(resource_offsets[i] as u32)?;
        if pak.version == 5 {
            file.write_u16::<LittleEndian>(0)?; // flags
        }
    }
    
    // Write alias table
    for alias in &pak.aliases {
        file.write_u16::<LittleEndian>(alias.id)?;
        file.write_u16::<LittleEndian>(alias.resource_id)?;
        file.write_u32::<LittleEndian>(0)?; // padding
    }
    
    // Write resource data
    file.write_all(&resource_data)?;
    
    file.flush()?;
    Ok(())
}

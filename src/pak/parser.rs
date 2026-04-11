use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Result, Context, bail};
use byteorder::{ReadBytesExt, LittleEndian};

use super::{PakFile, Resource, Alias};

/// Chrome Pak File Parser - Supports version 4 and 5 (matching C# implementation)
/// Format based on Chromium's pak file format:
/// Version 4:
///   [0-3] version (u32) = 4
///   [4-7] num_resources (u32)
///   [8] encoding (u8)
///   Then resource entries (num_resources + 1 entries, each 6 bytes: u16 id + u32 offset)
///
/// Version 5:
///   [0-3] version (u32) = 5
///   [4] encoding (u8)
///   [5-7] padding (3 bytes)
///   [8-9] num_resources (u16)
///   [10-11] num_aliases (u16)
///   Then resource entries (num_resources + 1 entries, each 6 bytes: u16 id + u32 offset)
///   Then alias entries (num_aliases entries, each 4 bytes: u16 id + u16 resource_idx)

pub fn parse_pak_file(path: &Path) -> Result<PakFile> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Cannot access pak file: {}", path.display()))?;

    if !metadata.is_file() {
        bail!("Path is not a file: {}", path.display());
    }

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            bail!("Permission denied: {}. Try running with sudo.", path.display());
        }
        Err(e) => return Err(e).with_context(|| format!("Failed to open pak file: {}", path.display()))?,
    };

    // Read version from first 4 bytes
    let version = file.read_u32::<LittleEndian>()?;

    eprintln!("[PAK DEBUG] File: {:?}, Version: {}", path.file_name().unwrap_or_default(), version);

    // Parse based on version
    match version {
        4 => parse_version4(&mut file, version),
        5 => parse_version5(&mut file, version),
        1..=3 => parse_version_legacy(&mut file, version),
        _ => {
            // Try as data pack format (version in first bytes, different structure)
            parse_data_pack(&mut file, version)
        }
    }
}

/// Parse Version 4 format
fn parse_version4(file: &mut fs::File, version: u32) -> Result<PakFile> {
    // Version 4 format:
    // [0-3] version (already read)
    // [4-7] num_resources (u32)
    // [8] encoding (u8)
    // Then resource entries (num_resources + 1 entries, each 6 bytes)

    let num_resources = file.read_u32::<LittleEndian>()? as usize;
    let encoding = file.read_u8()?;

    eprintln!("[PAK DEBUG] V4: num_resources={}, encoding={}", num_resources, encoding);

    // Sanity check
    if num_resources > 10000 {
        bail!("Too many resources: {} (max 10000)", num_resources);
    }

    // Read resource entries (num_resources + 1, last one is sentinel)
    let mut resources = Vec::new();
    for i in 0..num_resources + 1 {
        let id = file.read_u16::<LittleEndian>()?;
        let offset = file.read_u32::<LittleEndian>()?;
        
        eprintln!("[PAK DEBUG] V4 Entry {}: id={}, offset={}", i, id, offset);
        
        // Always push including sentinel — needed for last resource's size boundary
        resources.push((id, offset));
    }

    // Get file size for calculating resource sizes
    let file_size = file.seek(SeekFrom::End(0))?;

    // Convert to Resource structs (keep ALL entries including zero-length ones)
    let mut resource_vec = Vec::new();
    for i in 0..resources.len().saturating_sub(1) {
        let (id, offset) = resources[i];
        let (_, next_offset) = resources[i + 1];
        
        let data_size = if next_offset > offset {
            (next_offset - offset) as usize
        } else {
            0
        };

        if data_size > 100_000_000 {
            eprintln!("[PAK DEBUG] Warning: Skipping resource {} with absurd size {}", id, data_size);
            continue;
        }

        let data = if data_size > 0 {
            let mut buf = vec![0u8; data_size];
            file.seek(SeekFrom::Start(offset as u64))?;
            if let Err(e) = file.read_exact(&mut buf) {
                eprintln!("[PAK DEBUG] Warning: Could not read resource {}: {}", id, e);
                continue;
            }
            buf
        } else {
            Vec::new()
        };

        let mut resource = Resource {
            id,
            offset,
            data,
            width: None,
            height: None,
            format: None,
        };
        let _ = resource.parse_image_dimensions();
        resource_vec.push(resource);
    }

    eprintln!("[PAK DEBUG] V4: Loaded {} resources", resource_vec.len());

    Ok(PakFile {
        version,
        encoding,
        resources: resource_vec,
        aliases: Vec::new(),
    })
}

/// Parse Version 5 format
fn parse_version5(file: &mut fs::File, version: u32) -> Result<PakFile> {
    // Version 5 format:
    // [0-3] version (already read)
    // [4] encoding (u8)
    // [5-7] padding (3 bytes)
    // [8-9] num_resources (u16)
    // [10-11] num_aliases (u16)
    // Then resource entries (num_resources + 1 entries, each 6 bytes)
    // Then alias entries (num_aliases entries, each 4 bytes)

    let encoding = file.read_u8()?;
    
    // Skip 3 bytes padding
    let mut padding = [0u8; 3];
    file.read_exact(&mut padding)?;
    
    let num_resources = file.read_u16::<LittleEndian>()? as usize;
    let num_aliases = file.read_u16::<LittleEndian>()? as usize;

    eprintln!("[PAK DEBUG] V5: num_resources={}, num_aliases={}, encoding={}", num_resources, num_aliases, encoding);

    // Sanity check
    if num_resources > 10000 {
        bail!("Too many resources: {} (max 10000)", num_resources);
    }

    // Read resource entries (num_resources + 1, last one is sentinel)
    let mut resources = Vec::new();
    for i in 0..num_resources + 1 {
        let id = file.read_u16::<LittleEndian>()?;
        let offset = file.read_u32::<LittleEndian>()?;
        
        eprintln!("[PAK DEBUG] V5 Entry {}: id={}, offset={}", i, id, offset);
        
        // Always push including sentinel — needed for last resource's size boundary
        resources.push((id, offset));
    }

    // Read aliases
    let mut aliases = Vec::new();
    for _ in 0..num_aliases {
        let alias_id = file.read_u16::<LittleEndian>()?;
        let resource_idx = file.read_u16::<LittleEndian>()?;
        aliases.push(Alias {
            id: alias_id,
            resource_id: resource_idx,
            encoding: 0,
        });
    }

    // Get file size
    let file_size = file.seek(SeekFrom::End(0))?;

    // Convert to Resource structs (keep ALL entries including zero-length ones)
    let mut resource_vec = Vec::new();
    for i in 0..resources.len().saturating_sub(1) {
        let (id, offset) = resources[i];
        let (_, next_offset) = resources[i + 1];
        
        let data_size = if next_offset > offset {
            (next_offset - offset) as usize
        } else {
            0
        };

        if data_size > 100_000_000 {
            eprintln!("[PAK DEBUG] Warning: Skipping resource {} with absurd size {}", id, data_size);
            continue;
        }

        let data = if data_size > 0 {
            let mut buf = vec![0u8; data_size];
            file.seek(SeekFrom::Start(offset as u64))?;
            if let Err(e) = file.read_exact(&mut buf) {
                eprintln!("[PAK DEBUG] Warning: Could not read resource {}: {}", id, e);
                continue;
            }
            buf
        } else {
            Vec::new()
        };

        let mut resource = Resource {
            id,
            offset,
            data,
            width: None,
            height: None,
            format: None,
        };
        let _ = resource.parse_image_dimensions();
        resource_vec.push(resource);
    }

    eprintln!("[PAK DEBUG] V5: Loaded {} resources, {} aliases", resource_vec.len(), aliases.len());

    Ok(PakFile {
        version,
        encoding,
        resources: resource_vec,
        aliases,
    })
}

/// Parse legacy versions 1-3
fn parse_version_legacy(file: &mut fs::File, version: u32) -> Result<PakFile> {
    // Legacy format (version 1-3)
    // [0-3] version (already read)
    // [4-7] num_resources (u32)
    // [8] encoding (u8)
    // Then resource entries (num_resources + 1 entries, each 6 bytes)

    let num_resources = file.read_u32::<LittleEndian>()? as usize;
    let encoding = file.read_u8()?;

    eprintln!("[PAK DEBUG] Legacy V{}: num_resources={}, encoding={}", version, num_resources, encoding);

    if num_resources > 10000 {
        bail!("Too many resources: {} (max 10000)", num_resources);
    }

    let mut resources = Vec::new();
    for i in 0..num_resources + 1 {
        let id = file.read_u16::<LittleEndian>()?;
        let offset = file.read_u32::<LittleEndian>()?;
        
        // Always push including sentinel — needed for last resource's size boundary
        resources.push((id, offset));
    }

    let file_size = file.seek(SeekFrom::End(0))?;

    let mut resource_vec = Vec::new();
    for i in 0..resources.len().saturating_sub(1) {
        let (id, offset) = resources[i];
        let (_, next_offset) = resources[i + 1];
        
        let data_size = if next_offset > offset {
            (next_offset - offset) as usize
        } else {
            0
        };

        if data_size > 100_000_000 {
            continue;
        }

        let data = if data_size > 0 {
            let mut buf = vec![0u8; data_size];
            file.seek(SeekFrom::Start(offset as u64))?;
            if file.read_exact(&mut buf).is_err() {
                continue;
            }
            buf
        } else {
            Vec::new()
        };

        let mut resource = Resource {
            id,
            offset,
            data,
            width: None,
            height: None,
            format: None,
        };
        let _ = resource.parse_image_dimensions();
        resource_vec.push(resource);
    }

    eprintln!("[PAK DEBUG] Legacy: Loaded {} resources", resource_vec.len());

    Ok(PakFile {
        version,
        encoding,
        resources: resource_vec,
        aliases: Vec::new(),
    })
}

/// Parse Data Pack format (fallback for unrecognized versions)
fn parse_data_pack(file: &mut fs::File, version: u32) -> Result<PakFile> {
    // Try to read as a simple data pack
    // Assume: version (u32), encoding (u32), then 4-byte entries (u16 id + u16 offset)
    
    file.seek(SeekFrom::Start(4))?;
    let encoding = file.read_u32::<LittleEndian>()? as u8;
    let file_size = file.seek(SeekFrom::End(0))?;
    
    // Try to read entries until we hit the end or find an invalid entry
    let mut resources = Vec::new();
    file.seek(SeekFrom::Start(8))?;
    
    let mut i = 0;
    loop {
        if let (Ok(id), Ok(offset)) = (file.read_u16::<LittleEndian>(), file.read_u16::<LittleEndian>()) {
            if offset as u64 > file_size {
                break;
            }
            resources.push((id, offset as u32));
            i += 1;
            if i > 10000 {
                break;
            }
        } else {
            break;
        }
    }
    
    eprintln!("[PAK DEBUG] Data Pack: Read {} entries", resources.len());

    // Read resource data
    let mut resource_vec = Vec::new();
    for i in 0..resources.len().saturating_sub(1) {
        let (id, offset) = resources[i];
        let (_, next_offset) = resources[i + 1];
        
        let data_size = if next_offset > offset {
            (next_offset - offset) as usize
        } else {
            0
        };

        if data_size > 10_000_000 {
            continue;
        }

        let data = if data_size > 0 {
            let mut buf = vec![0u8; data_size];
            file.seek(SeekFrom::Start(offset as u64))?;
            if file.read_exact(&mut buf).is_err() {
                continue;
            }
            buf
        } else {
            Vec::new()
        };

        let mut resource = Resource {
            id,
            offset,
            data,
            width: None,
            height: None,
            format: None,
        };
        let _ = resource.parse_image_dimensions();
        resource_vec.push(resource);
    }

    Ok(PakFile {
        version,
        encoding,
        resources: resource_vec,
        aliases: Vec::new(),
    })
}

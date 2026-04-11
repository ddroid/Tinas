use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Result, Context};
use byteorder::{WriteBytesExt, LittleEndian};

use super::PakFile;

pub fn write_pak_file(pak: &PakFile, path: &Path) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create pak file: {}", path.display()))?;

    match pak.version {
        5 => write_version5(&mut file, pak),
        4 => write_version4(&mut file, pak),
        _ => write_version4(&mut file, pak), // fallback to v4 layout for legacy
    }
}

/// Write Version 5 format (matches parser exactly):
///   [0-3]   version     (u32 LE) = 5
///   [4]     encoding    (u8)
///   [5-7]   padding     (3 bytes, zeroes)
///   [8-9]   num_resources (u16 LE)
///   [10-11] num_aliases   (u16 LE)
///   Then (num_resources + 1) resource entries, each 6 bytes: u16 id + u32 offset
///     (last entry is sentinel: id=0, offset=end-of-data)
///   Then num_aliases alias entries, each 4 bytes: u16 alias_id + u16 resource_idx
///   Then resource data (packed, no alignment padding)
fn write_version5(file: &mut fs::File, pak: &PakFile) -> Result<()> {
    let num_resources = pak.resources.len();
    let num_aliases = pak.aliases.len();

    // Header: 4 + 1 + 3 + 2 + 2 = 12 bytes
    // Resource table: (num_resources + 1) * 6 bytes  (includes sentinel)
    // Alias table: num_aliases * 4 bytes
    let header_size: usize = 12;
    let resource_table_size = (num_resources + 1) * 6;
    let alias_table_size = num_aliases * 4;
    let data_start = header_size + resource_table_size + alias_table_size;

    // Pre-compute offsets for each resource's data
    let mut offsets: Vec<usize> = Vec::with_capacity(num_resources + 1);
    let mut cursor = data_start;
    for res in &pak.resources {
        offsets.push(cursor);
        cursor += res.data.len();
    }
    // Sentinel offset = end of all data
    offsets.push(cursor);

    // --- Write header ---
    file.write_u32::<LittleEndian>(5)?;           // version
    file.write_u8(pak.encoding)?;                  // encoding
    file.write_all(&[0u8; 3])?;                    // padding
    file.write_u16::<LittleEndian>(num_resources as u16)?;
    file.write_u16::<LittleEndian>(num_aliases as u16)?;

    // --- Write resource table (num_resources + 1 entries) ---
    for (i, res) in pak.resources.iter().enumerate() {
        file.write_u16::<LittleEndian>(res.id)?;
        file.write_u32::<LittleEndian>(offsets[i] as u32)?;
    }
    // Sentinel entry
    file.write_u16::<LittleEndian>(0)?;
    file.write_u32::<LittleEndian>(*offsets.last().unwrap() as u32)?;

    // --- Write alias table ---
    for alias in &pak.aliases {
        file.write_u16::<LittleEndian>(alias.id)?;
        file.write_u16::<LittleEndian>(alias.resource_id)?;
    }

    // --- Write resource data ---
    for res in &pak.resources {
        file.write_all(&res.data)?;
    }

    file.flush()?;
    Ok(())
}

/// Write Version 4 format (matches parser exactly):
///   [0-3]   version       (u32 LE) = 4
///   [4-7]   num_resources (u32 LE)
///   [8]     encoding      (u8)
///   Then (num_resources + 1) resource entries, each 6 bytes: u16 id + u32 offset
///     (last entry is sentinel: id=0, offset=end-of-data)
///   Then resource data (packed, no alignment padding)
fn write_version4(file: &mut fs::File, pak: &PakFile) -> Result<()> {
    let num_resources = pak.resources.len();

    // Header: 4 + 4 + 1 = 9 bytes
    // Resource table: (num_resources + 1) * 6
    let header_size: usize = 9;
    let resource_table_size = (num_resources + 1) * 6;
    let data_start = header_size + resource_table_size;

    let mut offsets: Vec<usize> = Vec::with_capacity(num_resources + 1);
    let mut cursor = data_start;
    for res in &pak.resources {
        offsets.push(cursor);
        cursor += res.data.len();
    }
    offsets.push(cursor);

    // --- Write header ---
    file.write_u32::<LittleEndian>(pak.version)?;  // version
    file.write_u32::<LittleEndian>(num_resources as u32)?;
    file.write_u8(pak.encoding)?;

    // --- Write resource table ---
    for (i, res) in pak.resources.iter().enumerate() {
        file.write_u16::<LittleEndian>(res.id)?;
        file.write_u32::<LittleEndian>(offsets[i] as u32)?;
    }
    // Sentinel
    file.write_u16::<LittleEndian>(0)?;
    file.write_u32::<LittleEndian>(*offsets.last().unwrap() as u32)?;

    // --- Write resource data ---
    for res in &pak.resources {
        file.write_all(&res.data)?;
    }

    file.flush()?;
    Ok(())
}

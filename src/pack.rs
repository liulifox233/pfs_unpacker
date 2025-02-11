use std::{
    io::Write,
    path::{Path, PathBuf},
};

use crate::ArtemisHeader;

struct ArtemisEntry<'a> {
    path: &'a PathBuf,
    offset: u32,
    file_size: u32,
}

pub fn pack(
    input_dir: &str,
    output_path: &str,
    version: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pack_version = version.unwrap_or(6);

    match pack_version {
        6 => pack_v6(input_dir, output_path)?,
        _ => todo!("pack version not implemented"),
    }

    Ok(())
}

fn pack_v6(input_dir: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entries = Path::new(input_dir)
        .read_dir()?
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();

    let mut artemis_entries = Vec::new();
    let mut offset = size_of::<ArtemisHeader>() as u32;
    let mut index_size = 0;
    for entry in &entries {
        let path = entry.strip_prefix(input_dir)?.to_str().unwrap().to_string();
        let path_len = path.len();
        let file_size = entry.metadata()?.len() as u32;

        artemis_entries.push(ArtemisEntry {
            path: entry,
            offset,
            file_size,
        });

        offset += file_size;
        index_size += (size_of::<u32>()// path length
            + path_len // path string
            + size_of::<u32>() // reversed
            + size_of::<u32>() // offset
            + size_of::<u32>()) as u32; // file size
    }

    let mut file = std::fs::File::create(output_path)?;

    file.write_all(b"pf")?; // magic
    file.write_all("6".as_bytes())?; // pack version
    file.write_all(&index_size.to_le_bytes())?; // index size (placeholder)
    file.write_all(&(entries.len() as u32).to_le_bytes())?; // file count

    for entry in &artemis_entries {
        let path = entry.path.strip_prefix(input_dir)?.to_str().unwrap();
        let path_len = path.len() as u32;

        file.write_all(&path_len.to_le_bytes())?; // path length
        file.write_all(path.as_bytes())?; // utf-8 path
        file.write_all(&[0, 0, 0, 0])?; // reserved
        file.write_all(&(entry.offset + index_size).to_le_bytes())?; // offset
        file.write_all(&entry.file_size.to_le_bytes())?; // file size
    }

    for entry in artemis_entries {
        let file_data = std::fs::read(entry.path)?;
        file.write_all(&file_data)?;
    }

    Ok(())
}

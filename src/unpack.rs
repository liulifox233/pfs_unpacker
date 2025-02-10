use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
    path::Path,
    str::from_utf8,
};

use crate::ArtemisEntry;
use crate::ArtemisHeader;
use crate::ARCHIVE_MAGIC;

use rayon::prelude::*;

pub fn unpack(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let header = read_header(&mut file)?;

    let index_size = header.index_size;
    let file_count = header.file_count;

    println!("index_size: {}", index_size);
    println!("file_count: {}", file_count);

    // Validate archive magic
    if header.magic != ARCHIVE_MAGIC {
        return Err("Invalid Artemis PFS archive!".into());
    }

    // Read index entries
    let entries = read_index(&mut file, &header)?;

    // Prepare output directory
    let base_path = Path::new(path);
    let output_dir = base_path.with_extension("");
    fs::create_dir_all(&output_dir)?;

    // Process files
    process_files(&mut file, entries, &output_dir, header)?;

    Ok(())
}

fn read_header(file: &mut File) -> Result<ArtemisHeader, Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 11];
    file.read_exact(&mut buffer)?;

    Ok(ArtemisHeader {
        magic: [buffer[0], buffer[1]],
        pack_version: buffer[2],
        index_size: u32::from_le_bytes([buffer[3], buffer[4], buffer[5], buffer[6]]),
        file_count: u32::from_le_bytes([buffer[7], buffer[8], buffer[9], buffer[10]]),
    })
}

fn read_index(
    file: &mut File,
    header: &ArtemisHeader,
) -> Result<Vec<ArtemisEntry>, Box<dyn std::error::Error>> {
    let mut entries = Vec::with_capacity(header.file_count as usize);

    for _ in 0..header.file_count {
        let mut path_len_buf = [0u8; 4];
        file.read_exact(&mut path_len_buf)?;
        let path_len = u32::from_le_bytes(path_len_buf) as usize;

        let mut path_buf = vec![0u8; path_len];
        file.read_exact(&mut path_buf)?;
        let path = from_utf8(&path_buf)?.to_string();

        // Skip reserved field
        file.seek(SeekFrom::Current(4))?;

        let mut offset_buf = [0u8; 4];
        file.read_exact(&mut offset_buf)?;
        let offset = u32::from_le_bytes(offset_buf);

        let mut size_buf = [0u8; 4];
        file.read_exact(&mut size_buf)?;
        let size = u32::from_le_bytes(size_buf);

        entries.push(ArtemisEntry { path, offset, size });
    }

    Ok(entries)
}

fn process_files(
    file: &mut File,
    entries: Vec<ArtemisEntry>,
    output_dir: &Path,
    header: ArtemisHeader,
) -> Result<(), Box<dyn std::error::Error>> {
    let xor_key = if header.pack_version == b'8' {
        let mut hasher = Sha1::new();
        let mut index_data = vec![0u8; header.index_size as usize];
        file.seek(SeekFrom::Start(
            (size_of::<ArtemisHeader>() - 4).try_into().unwrap(),
        ))?;
        file.read_exact(&mut index_data)?;
        hasher.update(&index_data);
        hasher.finalize().to_vec()
    } else {
        Vec::new()
    };

    entries
        .par_iter()
        .filter(|entry| entry.offset != 0)
        .for_each(|entry| {
            #[cfg(not(target_os = "windows"))]
            let entry_path = entry.path.replace("\\", "/");

            #[cfg(target_os = "windows")]
            let entry_path = entry.path;

            let entry_path = Path::new(&entry_path);
            let output_path = output_dir.join(entry_path);

            println!("processing: {}", output_path.display());
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).expect("Failed to create parent directory");
            }

            let mut buffer = vec![0u8; entry.size as usize];
            file.read_exact_at(&mut buffer, entry.offset as u64)
                .expect("Failed to read file data");

            if header.pack_version == b'8' {
                xor_crypt(&mut buffer, &xor_key);
            }

            let mut output_file = File::create(&output_path).expect("Failed to create output file");
            output_file.write_all(&buffer).unwrap();
        });

    Ok(())
}

fn xor_crypt(data: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return;
    }

    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
}

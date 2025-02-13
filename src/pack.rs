use crate::{xor_crypt, ArtemisHeader, ARCHIVE_MAGIC};
use sha1::{Digest, Sha1};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
use walkdir::WalkDir;

struct ArtemisEntry<'a> {
    path: &'a PathBuf,
    offset: u32,
    file_size: u32,
    index: u32,
}

pub fn pack(input_dir: &str, output_path: &str, version: Option<u8>) {
    let pack_version = version.unwrap_or(6);

    match pack_version {
        6 => pack_v6(input_dir, output_path),
        8 => pack_v8(input_dir, output_path),
        _ => todo!("pack version not implemented"),
    };
}

fn pack_v6(input_dir: &str, output_path: &str) {
    let entries: Vec<_> = WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
        .collect();

    let file_count = entries.len();

    let mut artemis_entries = Vec::new();
    let mut offset = 0; // raw file offset
    let mut index_size = (
        size_of::<u32>()
        // file count (4)
    ) as u32;
    for entry in &entries {
        let path = entry
            .strip_prefix(input_dir)
            .expect("failed to strip prefix from entry path")
            .to_str()
            .unwrap()
            .to_string();
        let path_len = path.len();
        let file_size = entry
            .metadata()
            .expect("failed to get metadata for entry")
            .len() as u32;

        index_size += (
            size_of::<u32>()// path length
            + path_len
            // path string
        ) as u32;

        artemis_entries.push(ArtemisEntry {
            path: entry,
            offset,
            file_size,
            index: index_size,
        });

        index_size += (
            size_of::<u32>() // reserved
        + size_of::<u32>() // offset
        + size_of::<u32>()
            // file size
        ) as u32;

        offset += file_size;
    }
    let table_pos = index_size;
    index_size += (
        file_count * (
        size_of::<u32>() // index offset (4)
        + size_of::<u32>() // reserved (4)
    )
    + size_of::<u32>() // file count (+1) (4)
    + size_of::<u32>() // reserved (4)
    + size_of::<u32>() // reserved (4)
    + size_of::<u32>()
        // table pos (4)
    ) as u32;

    let mut file = std::fs::File::create(output_path).expect("failed to create output file");

    file.write_all(&ARCHIVE_MAGIC).unwrap(); // magic
    file.write_all(b"6").unwrap(); // pack version
    file.write_all(&index_size.to_le_bytes()).unwrap(); // index size
    file.write_all(&(file_count as u32).to_le_bytes()).unwrap(); // file count

    for entry in &artemis_entries {
        let path = entry
            .path
            .strip_prefix(input_dir)
            .expect("failed to strip prefix from entry path")
            .to_str()
            .unwrap();
        let path_len = path.len() as u32;

        #[cfg(target_os = "windows")]
        let path_str = path.to_string();

        #[cfg(not(target_os = "windows"))]
        let path_str = path.replace("/", "\\");

        file.write_all(&path_len.to_le_bytes()).unwrap(); // path length
        file.write_all(path_str.as_bytes()).unwrap(); // utf-8 path
        file.write_all(&[0; 4]).unwrap(); // reserved
        file.write_all(
            &(entry.offset + index_size + (size_of::<ArtemisHeader>() - size_of::<u32>()) as u32)
                .to_le_bytes(),
        )
        .unwrap(); // offset
        file.write_all(&entry.file_size.to_le_bytes()).unwrap(); // file size
    }

    file.write_all(&(file_count as u32 + 1_u32).to_le_bytes())
        .unwrap();

    for entry in &artemis_entries {
        file.write_all(&entry.index.to_le_bytes()).unwrap();
        file.write_all(&[0; 4]).unwrap(); // reserved
    }

    file.write_all(&[0; 8]).unwrap(); // reserved
    file.write_all(&table_pos.to_le_bytes()).unwrap(); // table pos

    for entry in &artemis_entries {
        let mut raw_file = std::fs::File::open(entry.path).unwrap();
        std::io::copy(&mut raw_file, &mut file).unwrap();
    }
}

fn pack_v8(input_dir: &str, output_path: &str) {
    let entries: Vec<_> = WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
        .collect();

    let file_count = entries.len();

    let mut artemis_entries = Vec::new();
    let mut offset = 0; // raw file offset
    let mut index_size = (
        size_of::<u32>()
        // file count (4)
    ) as u32;
    for entry in &entries {
        let path = entry
            .strip_prefix(input_dir)
            .expect("failed to strip prefix from entry path")
            .to_str()
            .unwrap()
            .to_string();
        let path_len = path.len();
        let file_size = entry
            .metadata()
            .expect("failed to get metadata for entry")
            .len() as u32;

        index_size += (
            size_of::<u32>()// path length
            + path_len
            // path string
        ) as u32;

        artemis_entries.push(ArtemisEntry {
            path: entry,
            offset,
            file_size,
            index: index_size,
        });

        index_size += (
            size_of::<u32>() // reserved
        + size_of::<u32>() // offset
        + size_of::<u32>()
            // file size
        ) as u32;

        offset += file_size;
    }
    let table_pos = index_size;
    index_size += (
        file_count * (
        size_of::<u32>() // index offset (4)
        + size_of::<u32>() // reserved (4)
    )
    + size_of::<u32>() // file count (+1) (4)
    + size_of::<u32>() // reserved (4)
    + size_of::<u32>() // reserved (4)
    + size_of::<u32>()
        // table pos (4)
    ) as u32;

    let mut file = std::fs::File::create(output_path).expect("failed to create output file");

    let mut hasher = Sha1::new();

    file.write_all(&ARCHIVE_MAGIC).unwrap(); // magic
    file.write_all(b"8").unwrap(); // pack version
    file.write_all(&index_size.to_le_bytes()).unwrap(); // index size
    hash_and_write(&mut file, &mut hasher, &(file_count as u32).to_le_bytes()); // file count

    for entry in &artemis_entries {
        let path = entry
            .path
            .strip_prefix(input_dir)
            .expect("failed to strip prefix from entry path")
            .to_str()
            .unwrap();
        let path_len = path.len() as u32;

        #[cfg(target_os = "windows")]
        let path_str = path.to_string();

        #[cfg(not(target_os = "windows"))]
        let path_str = path.replace("/", "\\");

        hash_and_write(&mut file, &mut hasher, &path_len.to_le_bytes()); // path length
        hash_and_write(&mut file, &mut hasher, path_str.as_bytes()); // utf-8 path
        hash_and_write(&mut file, &mut hasher, &[0; 4]); // reserved
        hash_and_write(
            &mut file,
            &mut hasher,
            &(entry.offset + index_size + (size_of::<ArtemisHeader>() - size_of::<u32>()) as u32)
                .to_le_bytes(),
        ); // offset
        hash_and_write(&mut file, &mut hasher, &entry.file_size.to_le_bytes()); // file size
    }

    hash_and_write(
        &mut file,
        &mut hasher,
        &(file_count as u32 + 1_u32).to_le_bytes(),
    );

    for entry in &artemis_entries {
        hash_and_write(&mut file, &mut hasher, &entry.index.to_le_bytes());
        hash_and_write(&mut file, &mut hasher, &[0; 4]); // reserved
    }

    hash_and_write(&mut file, &mut hasher, &[0; 8]); // reserved
    hash_and_write(&mut file, &mut hasher, &table_pos.to_le_bytes()); // table pos

    let xor_key = hasher.finalize().to_vec();

    for entry in &artemis_entries {
        let mut raw_file = std::fs::File::open(entry.path).unwrap();
        let mut buf = vec![0u8; 1024];
        while let Ok(read) = raw_file.read(&mut buf) {
            if read == 0 {
                break;
            }
            println!("read: {}", read);
            xor_crypt(&mut buf[..read], &xor_key);
            file.write_all(&buf[..read]).unwrap();
        }
    }
}

fn hash_and_write(file: &mut std::fs::File, hasher: &mut Sha1, data: &[u8]) {
    hasher.update(data);
    file.write_all(data).unwrap();
}

use crate::{get_path_str, xor_crypt, ArtemisHeader, ARCHIVE_MAGIC};
use sha1::{Digest, Sha1};
use std::{
    fs::File,
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
    let entries: Vec<_> = WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
        .collect();
    let file_count = entries.len();

    let (index_size, artemis_entries, table_pos) =
        calculate_index_size_and_entries(input_dir, &entries);

    let mut pack = match version {
        Some(6) | None => Pack::new(
            File::create(output_path).expect("failed to create output file"),
            None,
        ),
        Some(8) => Pack::new(
            File::create(output_path).expect("failed to create output file"),
            Some(Sha1::new()),
        ),
        _ => panic!("unsupported pack version"),
    };

    pack.write_head(version);
    pack.write_index_size(index_size);
    pack.write_file_count(file_count);

    for entry in &artemis_entries {
        pack.write_path(entry.path, entry.offset, index_size, input_dir);
        pack.write_file_size(entry.file_size);
    }

    pack.write_file_count(file_count + 1);
    pack.write_index(&artemis_entries);
    pack.write_reserved(8);
    pack.write_table_pos(table_pos);
    pack.write_files(&artemis_entries);
}

struct Pack {
    file: File,
    hasher: Option<Sha1>,
}

impl Pack {
    fn new(file: File, hasher: Option<Sha1>) -> Self {
        Self { file, hasher }
    }

    fn write_all(&mut self, data: &[u8], hash: bool) {
        self.file.write_all(data).unwrap();
        if let Some(hasher) = &mut self.hasher {
            if hash {
                hasher.update(data);
            }
        }
    }

    fn write_head(&mut self, version: Option<u8>) {
        self.write_all(&ARCHIVE_MAGIC, false);
        self.write_all(version.unwrap_or(6).to_string().as_bytes(), false);
    }

    fn write_index_size(&mut self, index_size: u32) {
        self.write_all(&index_size.to_le_bytes(), false);
    }

    fn write_file_size(&mut self, file_size: u32) {
        self.write_all(&file_size.to_le_bytes(), true);
    }

    fn write_file_count(&mut self, file_count: usize) {
        self.write_all(&(file_count as u32).to_le_bytes(), true);
    }

    fn write_index(&mut self, entries: &[ArtemisEntry]) {
        for entry in entries {
            self.write_all(&entry.index.to_le_bytes(), true);
            self.write_reserved(4);
        }
    }

    fn write_table_pos(&mut self, table_pos: u32) {
        self.write_all(&table_pos.to_le_bytes(), true);
    }

    fn write_reserved(&mut self, len: usize) {
        self.write_all(&vec![0; len], true);
    }

    fn write_path(&mut self, path: &PathBuf, offset: u32, index_size: u32, input_dir: &str) {
        let path = path
            .strip_prefix(input_dir)
            .expect("failed to strip prefix from entry path")
            .to_str()
            .unwrap();

        let path_len = path.len() as u32;

        let path_str = get_path_str(path);

        self.write_all(&path_len.to_le_bytes(), true);
        self.write_all(path_str.as_bytes(), true);
        self.write_reserved(4);
        self.write_all(
            &(offset + index_size + (size_of::<ArtemisHeader>() - size_of::<u32>()) as u32)
                .to_le_bytes(),
            true,
        );
    }

    fn write_files(&mut self, entries: &[ArtemisEntry]) {
        match self.hasher {
            Some(ref mut hasher) => {
                let xor_key = hasher.clone().finalize().to_vec();
                for entry in entries {
                    let mut raw_file = std::fs::File::open(entry.path).unwrap();
                    let mut buf = vec![0u8; 1024];
                    while let Ok(read) = raw_file.read(&mut buf) {
                        if read == 0 {
                            break;
                        }
                        xor_crypt(&mut buf[..read], &xor_key);
                        self.write_all(&buf[..read], true);
                    }
                }
            }
            None => {
                for entry in entries {
                    let mut raw_file = std::fs::File::open(entry.path).unwrap();
                    std::io::copy(&mut raw_file, &mut self.file).unwrap();
                }
            }
        }
    }
}

fn calculate_index_size_and_entries<'a>(
    input_dir: &'a str,
    entries: &'a [PathBuf],
) -> (u32, Vec<ArtemisEntry<'a>>, u32) {
    let file_count = entries.len();
    let mut artemis_entries = Vec::new();
    let mut offset = 0; // raw file offset
    let mut index_size = size_of::<u32>() as u32; // file count (4)

    for entry in entries {
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

        index_size += (size_of::<u32>() + path_len) as u32; // path length + path string

        artemis_entries.push(ArtemisEntry {
            path: entry,
            offset,
            file_size,
            index: index_size,
        });

        index_size += (size_of::<u32>() + size_of::<u32>() + size_of::<u32>()) as u32; // reserved + offset + file size

        offset += file_size;
    }

    let table_pos = index_size;
    index_size += (file_count * (size_of::<u32>() + size_of::<u32>())
        + size_of::<u32>()
        + size_of::<u32>()
        + size_of::<u32>()
        + size_of::<u32>()) as u32; // index offset + reserved + file count + reserved + reserved + table pos

    assert_eq!(index_size, 83);

    (index_size, artemis_entries, table_pos)
}

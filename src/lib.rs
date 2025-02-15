pub mod pack;
pub mod unpack;

const ARCHIVE_MAGIC: [u8; 2] = [0x70, 0x66];
const ARCHIVE_MAGIC_SIZE: usize = ARCHIVE_MAGIC.len();

#[repr(packed)]
struct ArtemisHeader {
    magic: [u8; ARCHIVE_MAGIC_SIZE],
    pack_version: u8,
    index_size: u32,
    file_count: u32,
}

/// Returns the path string with the correct path separator for the current OS.
pub fn get_path_str(path: &str) -> String {
    #[cfg(target_os = "windows")]
    let path_str = path.to_string();

    #[cfg(not(target_os = "windows"))]
    let path_str = path.replace("/", "\\");

    path_str
}

/// XOR encrypts or decrypts data using the provided key.
pub fn xor_crypt(data: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return;
    }

    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
}

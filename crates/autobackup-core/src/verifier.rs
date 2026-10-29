use md5::{Digest, Md5};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const HASH_CHUNK_SIZE: usize = 1024 * 1024;

pub fn file_md5(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();
    let mut buffer = [0u8; HASH_CHUNK_SIZE];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_copy(src: &Path, dst: &Path) -> std::io::Result<bool> {
    let src_meta = src.metadata()?;
    let dst_meta = dst.metadata()?;
    if src_meta.len() != dst_meta.len() {
        return Ok(false);
    }
    Ok(file_md5(src)? == file_md5(dst)?)
}

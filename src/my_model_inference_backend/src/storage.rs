// src/my_model_inference_backend/src/storage.rs
use prost::bytes::Bytes;
use std::io::Write;

pub fn bytes(filename: &str) -> Bytes {
    std::fs::read(filename).unwrap().into()
}

pub fn append_bytes(filename: &str, bytes: Vec<u8>) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename)
        .unwrap();
    file.write_all(&bytes).unwrap();
}

pub fn clear_bytes(filename: &str) {
    let _ = std::fs::remove_file(filename);
}

pub fn bytes_length(filename: &str) -> usize {
    std::fs::metadata(filename).map(|metadata| metadata.len() as usize).unwrap_or(0)
}
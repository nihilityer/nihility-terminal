use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha512};
use tracing::debug;

use crate::config::ORT_LIB_PATH;

const ORT_LIB_DOWNLOAD_FILE_NAME: &str = "onnxruntime-win-x64-1.16.3.zip";
const ORT_LIB_DOWNLOAD_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.16.3/onnxruntime-win-x64-1.16.3.zip";
const ORT_LIB_ZIP_FILE_HASH: &str = "855cc3f9f354c2acd472a066118a86b84c7b8940f098acef2d2d239af0a3c69fb32026d4ba4b86c46848a849f963691f94751df9004efa817fdcea48ec9cb4e6";
const ORT_LIB_NAME_IN_ZIP: &str = "onnxruntime-win-x64-1.16.3/lib/onnxruntime.dll";

pub fn check() -> Result<()> {
    if !Path::new(ORT_LIB_PATH).exists() {
        download_ort_lib()?;
    }
    Ok(())
}

fn download_ort_lib() -> Result<()> {
    extract_lib(&download_file(ORT_LIB_DOWNLOAD_URL, ORT_LIB_ZIP_FILE_HASH)?[..])
}

fn download_file(source_url: &str, source_sha512: &str) -> Result<Vec<u8>> {
    let resp = ureq::get(source_url)
        .timeout(std::time::Duration::from_secs(1800))
        .call()?;

    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .expect("Content-Length header should be present on archive response");
    let mut reader = resp.into_reader();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    debug!("Download file len: {:?}", &len);
    assert_eq!(buffer.len(), len);

    let mut hasher = Sha512::new();
    hasher.update(&buffer);
    let result = hasher.finalize();
    let s = hex::encode(result);
    debug!("Download file sha512: {:?}", &s);
    assert_eq!(s, source_sha512);
    Ok(buffer)
}

fn extract_lib(buffer: &[u8]) -> Result<()> {
    let lib_path = Path::new(ORT_LIB_PATH);
    let lib_dir = lib_path.parent().expect("ORT_LIB_PATH Error");
    create_dir_all(lib_dir)?;
    let zip_file_path = lib_dir.join(ORT_LIB_DOWNLOAD_FILE_NAME);
    let mut zip_file = File::options()
        .write(true)
        .read(true)
        .create_new(true)
        .open(zip_file_path)?;
    zip_file.write_all(&buffer[..])?;
    let mut zip = zip::ZipArchive::new(zip_file)?;
    let mut buffer = Vec::<u8>::new();
    for file_name in zip.file_names() {
        debug!("Zip File Inner File Name: {}", file_name);
    }
    zip.by_name(ORT_LIB_NAME_IN_ZIP)?.read_to_end(&mut buffer)?;
    File::options()
        .write(true)
        .create_new(true)
        .open(ORT_LIB_PATH)?
        .write_all(&buffer[..])?;
    Ok(())
}

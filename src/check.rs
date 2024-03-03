use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha512};
use tracing::debug;

use crate::config::ORT_LIB_PATH;

const _SENTENCE_TRANSFORMERS_TOKENIZER_URL: &str = "https://www.modelscope.cn/api/v1/models/Xorbits/bge-small-zh/repo?Revision=master&FilePath=tokenizer.json";
const _SENTENCE_TRANSFORMERS_MODEL_URL: &str = "https://www.modelscope.cn/api/v1/models/Xorbits/bge-small-zh/repo?Revision=master&FilePath=pytorch_model.bin";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ORT_LIB_DOWNLOAD_FILE_NAME: &str = "onnxruntime-win-x64-1.17.1.zip";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ORT_LIB_NAME_IN_ZIP: &str = "onnxruntime-win-x64-1.17.1/lib/onnxruntime.dll";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ORT_LIB_DOWNLOAD_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.17.1/onnxruntime-win-x64-1.17.1.zip";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ORT_LIB_ZIP_FILE_SIZE: usize = 61193481;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const ORT_LIB_ZIP_FILE_HASH: &str = "3d0d5fb5de7f0fa35d63a71578702746f84d04335428191e20b931d6d272d5793d9169732ee1f9f42cd96dc3e36a833d6c56cb07aadb4f14202a9119f67ea903";
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ORT_LIB_DOWNLOAD_FILE_NAME: &str = "onnxruntime-win-x86-1.17.1.zip";
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ORT_LIB_NAME_IN_ZIP: &str = "onnxruntime-win-x86-1.17.1/lib/onnxruntime.dll";
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ORT_LIB_DOWNLOAD_URL: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.17.1/onnxruntime-win-x86-1.17.1.zip";
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ORT_LIB_ZIP_FILE_SIZE: usize = 60244610;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const ORT_LIB_ZIP_FILE_HASH: &str = "974e24550c0d4c54b2672b4626b978928d554651b8c715d1d44412b5ddb751d724cf8179e0e58de975c8a2b6819212c024172b24fd08b8f97d601d36f1a601f0";

pub fn check() -> Result<()> {
    if !Path::new(ORT_LIB_PATH).exists() {
        download_ort_lib()?;
    }
    Ok(())
}

fn download_ort_lib() -> Result<()> {
    extract_lib(
        &download_file(
            ORT_LIB_DOWNLOAD_URL,
            ORT_LIB_ZIP_FILE_SIZE,
            ORT_LIB_ZIP_FILE_HASH,
        )?[..],
    )
}

fn download_file(source_url: &str, source_size: usize, source_sha512: &str) -> Result<Vec<u8>> {
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
    debug!("Download File Size: {:?}", &len);
    assert_eq!(buffer.len(), len);
    assert_eq!(buffer.len(), source_size);

    let mut hasher = Sha512::new();
    hasher.update(&buffer);
    let result = hasher.finalize();
    let s = hex::encode(result);
    debug!("Download File Sha512: {:?}", &s);
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
    zip_file.write_all(buffer)?;
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

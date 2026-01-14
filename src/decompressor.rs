use anyhow::{Context, Result};
use flate2::read::{GzDecoder, ZlibDecoder};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tar::Archive;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    Gzip,
    Z,
    None,
}

#[derive(Debug)]
pub enum FileType {
    Tar,
    CompressedFile,
    PlainFile,
}

pub fn detect_file_type<P: AsRef<Path>>(file_path: P) -> Result<FileType> {
    let path = file_path.as_ref();
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .context("Invalid filename")?;
    
    let lower = filename.to_lowercase();
    
    if lower.ends_with(".tar") || lower.ends_with(".tar.gz") || lower.ends_with(".tgz") || lower.ends_with(".tar.z") {
        Ok(FileType::Tar)
    } else if lower.ends_with(".gz") || lower.ends_with(".z") {
        Ok(FileType::CompressedFile)
    } else {
        Ok(FileType::PlainFile)
    }
}

pub fn detect_file_compression(filename: &str) -> CompressionType {
    let lower = filename.to_lowercase();
    
    if lower.ends_with(".gz") {
        CompressionType::Gzip
    } else if lower.ends_with(".z") {
        CompressionType::Z
    } else {
        CompressionType::None
    }
}

pub fn extract_file_from_tar<P: AsRef<Path>>(
    tar_path: P,
    target_filename: &str,
    mut uploader: impl FnMut(&[u8], bool) -> Result<()>,
) -> Result<u64> {
    let tar_path = tar_path.as_ref();
    info!("Extracting '{}' from tar: {}", target_filename, tar_path.display());
    
    let file = File::open(tar_path)
        .with_context(|| format!("Failed to open tar file: {}", tar_path.display()))?;
    let reader = BufReader::new(file);
    
    let mut tar = Archive::new(reader);
    
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        
        if let Some(filename) = path.file_name() {
            if filename == target_filename || path.to_string_lossy().contains(target_filename) {
                debug!("Found file in tar: {}", path.display());
                
                let size = entry.size();
                let filename_str = path.to_string_lossy().to_string();
                let compression = detect_file_compression(&filename_str);
                debug!("Detected compression type for '{}': {:?}", filename_str, compression);
                
                let mut reader: Box<dyn Read> = match compression {
                    CompressionType::Gzip => {
                        debug!("Creating Gzip decoder");
                        Box::new(GzDecoder::new(entry))
                    }
                    CompressionType::Z => {
                        debug!("Creating Z decoder");
                        Box::new(ZlibDecoder::new(entry))
                    }
                    CompressionType::None => {
                        debug!("No compression, using raw reader");
                        Box::new(entry)
                    }
                };
                
                let mut buffer = vec![0u8; 1024 * 1024];
                let mut total_size = 0u64;
                
                loop {
                    let bytes_read = reader.read(&mut buffer)
                        .context("Failed to read from decompressor")?;
                    
                    if bytes_read == 0 {
                        break;
                    }
                    
                    total_size += bytes_read as u64;
                    uploader(&buffer[..bytes_read], false)?;
                }
                
                uploader(&[], true)?;
                
                return Ok(total_size);
            }
        }
    }
    
    Err(anyhow::anyhow!(
        "File '{}' not found in tar archive: {}",
        target_filename,
        tar_path.display()
    ))
}

pub fn extract_compressed_file<P: AsRef<Path>>(
    file_path: P,
    mut uploader: impl FnMut(&[u8], bool) -> Result<()>,
) -> Result<u64> {
    let file_path = file_path.as_ref();
    info!("Extracting compressed file: {}", file_path.display());
    
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open compressed file: {}", file_path.display()))?;
    let reader = BufReader::new(file);
    
    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .context("Invalid filename")?;
    
    let compression = detect_file_compression(filename);
    debug!("Detected compression type for '{}': {:?}", filename, compression);
    
    let mut reader: Box<dyn Read> = match compression {
        CompressionType::Gzip => {
            debug!("Creating Gzip decoder");
            Box::new(GzDecoder::new(reader))
        }
        CompressionType::Z => {
            debug!("Creating Z decoder");
            Box::new(ZlibDecoder::new(reader))
        }
        CompressionType::None => {
            debug!("No compression, using raw reader");
            Box::new(reader)
        }
    };
    
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total_size = 0u64;
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .context("Failed to read from decompressor")?;
        
        if bytes_read == 0 {
            break;
        }
        
        total_size += bytes_read as u64;
        uploader(&buffer[..bytes_read], false)?;
    }
    
    uploader(&[], true)?;
    
    Ok(total_size)
}

pub fn extract_plain_file<P: AsRef<Path>>(
    file_path: P,
    mut uploader: impl FnMut(&[u8], bool) -> Result<()>,
) -> Result<u64> {
    let file_path = file_path.as_ref();
    info!("Reading plain file: {}", file_path.display());
    
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    let mut reader = BufReader::new(file);
    
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut total_size = 0u64;
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .context("Failed to read from file")?;
        
        if bytes_read == 0 {
            break;
        }
        
        total_size += bytes_read as u64;
        uploader(&buffer[..bytes_read], false)?;
    }
    
    uploader(&[], true)?;
    
    Ok(total_size)
}

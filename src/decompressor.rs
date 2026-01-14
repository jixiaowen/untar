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

pub struct ExtractedFile {
    pub name: String,
    pub size: u64,
    pub reader: Box<dyn Read + Send>,
}

pub fn extract_file<P: AsRef<Path>>(
    file_path: P,
    target_filename: Option<&str>,
) -> Result<ExtractedFile> {
    let file_path = file_path.as_ref();
    let file_type = detect_file_type(file_path)?;
    
    match file_type {
        FileType::Tar => {
            if let Some(target) = target_filename {
                extract_file_from_tar(file_path, target)
            } else {
                Err(anyhow::anyhow!("Target filename required for tar files"))
            }
        }
        FileType::CompressedFile => {
            extract_compressed_file(file_path)
        }
        FileType::PlainFile => {
            extract_plain_file(file_path)
        }
    }
}

pub fn extract_file_from_tar<P: AsRef<Path>>(
    tar_path: P,
    target_filename: &str,
) -> Result<ExtractedFile> {
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
                
                let reader: Box<dyn Read + Send> = match compression {
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
                
                return Ok(ExtractedFile {
                    name: filename_str,
                    size,
                    reader,
                });
            }
        }
    }
    
    Err(anyhow::anyhow!(
        "File '{}' not found in tar archive: {}",
        target_filename,
        tar_path.display()
    ))
}

fn extract_compressed_file<P: AsRef<Path>>(file_path: P) -> Result<ExtractedFile> {
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
    
    let reader: Box<dyn Read + Send> = match compression {
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
    
    let metadata = std::fs::metadata(file_path)?;
    
    Ok(ExtractedFile {
        name: filename.to_string(),
        size: metadata.len(),
        reader,
    })
}

fn extract_plain_file<P: AsRef<Path>>(file_path: P) -> Result<ExtractedFile> {
    let file_path = file_path.as_ref();
    info!("Reading plain file: {}", file_path.display());
    
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    let reader = BufReader::new(file);
    
    let filename = file_path.file_name()
        .and_then(|n| n.to_str())
        .context("Invalid filename")?;
    
    let metadata = std::fs::metadata(file_path)?;
    
    Ok(ExtractedFile {
        name: filename.to_string(),
        size: metadata.len(),
        reader: Box::new(reader),
    })
}

pub fn extract_all_files_from_tar<P: AsRef<Path>>(
    tar_path: P,
) -> Result<Vec<ExtractedFile>> {
    let tar_path = tar_path.as_ref();
    info!("Extracting all files from tar: {}", tar_path.display());
    
    let file = File::open(tar_path)
        .with_context(|| format!("Failed to open tar file: {}", tar_path.display()))?;
    let reader = BufReader::new(file);
    
    let mut tar = Archive::new(reader);
    let mut files = Vec::new();
    
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let size = entry.size();
        
        let filename_str = path.to_string_lossy().to_string();
        let compression = detect_file_compression(&filename_str);
        debug!("Processing '{}': compression type: {:?}", filename_str, compression);
        
        let reader: Box<dyn Read + Send> = match compression {
            CompressionType::Gzip => {
                Box::new(GzDecoder::new(entry))
            }
            CompressionType::Z => {
                Box::new(ZlibDecoder::new(entry))
            }
            CompressionType::None => {
                Box::new(entry)
            }
        };
        
        files.push(ExtractedFile {
            name: filename_str,
            size,
            reader,
        });
    }
    
    Ok(files)
}

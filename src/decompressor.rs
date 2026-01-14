use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tar::Archive;
use tracing::{debug, info};

#[derive(Debug)]
pub enum CompressionType {
    Gzip,
    Z,
    None,
}

pub fn detect_compression<P: AsRef<Path>>(tar_path: P) -> Result<CompressionType> {
    let path = tar_path.as_ref();
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .context("Invalid tar filename")?;
    
    if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        Ok(CompressionType::Gzip)
    } else if filename.ends_with(".tar.Z") {
        Ok(CompressionType::Z)
    } else if filename.ends_with(".tar") {
        Ok(CompressionType::None)
    } else {
        Ok(CompressionType::Gzip)
    }
}

pub struct ExtractedFile {
    pub name: String,
    pub size: u64,
    pub data: Vec<u8>,
}

pub fn extract_file_from_tar<P: AsRef<Path>>(
    tar_path: P,
    target_filename: &str,
) -> Result<ExtractedFile> {
    let tar_path = tar_path.as_ref();
    info!("Extracting '{}' from tar: {}", target_filename, tar_path.display());
    
    let compression = detect_compression(tar_path)?;
    debug!("Detected compression type: {:?}", compression);
    
    let file = File::open(tar_path)
        .with_context(|| format!("Failed to open tar file: {}", tar_path.display()))?;
    let reader = BufReader::new(file);
    
    let archive: Box<dyn Read> = match compression {
        CompressionType::Gzip => {
            Box::new(GzDecoder::new(reader))
        }
        CompressionType::Z => {
            Box::new(decompress_z(reader)?)
        }
        CompressionType::None => {
            Box::new(reader)
        }
    };
    
    let mut tar = Archive::new(archive);
    
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        
        if let Some(filename) = path.file_name() {
            if filename == target_filename || path.to_string_lossy().contains(target_filename) {
                debug!("Found file in tar: {}", path.display());
                
                let mut data = Vec::new();
                let size = entry.size();
                entry.read_to_end(&mut data)?;
                
                return Ok(ExtractedFile {
                    name: path.to_string_lossy().to_string(),
                    size,
                    data,
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

pub fn extract_all_files_from_tar<P: AsRef<Path>>(
    tar_path: P,
) -> Result<Vec<ExtractedFile>> {
    let tar_path = tar_path.as_ref();
    info!("Extracting all files from tar: {}", tar_path.display());
    
    let compression = detect_compression(tar_path)?;
    debug!("Detected compression type: {:?}", compression);
    
    let file = File::open(tar_path)
        .with_context(|| format!("Failed to open tar file: {}", tar_path.display()))?;
    let reader = BufReader::new(file);
    
    let archive: Box<dyn Read> = match compression {
        CompressionType::Gzip => {
            Box::new(GzDecoder::new(reader))
        }
        CompressionType::Z => {
            Box::new(decompress_z(reader)?)
        }
        CompressionType::None => {
            Box::new(reader)
        }
    };
    
    let mut tar = Archive::new(archive);
    let mut files = Vec::new();
    
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let size = entry.size();
        
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        
        files.push(ExtractedFile {
            name: path.to_string_lossy().to_string(),
            size,
            data,
        });
    }
    
    Ok(files)
}

fn decompress_z<R: Read>(reader: R) -> Result<impl Read> {
    let mut decoder = libz_sys::ZStream::default();
    
    decoder.next_in = std::ptr::null_mut();
    decoder.avail_in = 0;
    
    let result = unsafe {
        libz_sys::inflateInit2_(
            &mut decoder as *mut _ as *mut libz_sys::z_stream,
            -15,
            libz_sys::zlibVersion(),
            std::mem::size_of::<libz_sys::z_stream>() as i32,
        )
    };
    
    if result != 0 {
        return Err(anyhow::anyhow!("Failed to initialize Z decompressor"));
    }
    
    Ok(ZDecoder {
        reader,
        decoder,
        finished: false,
    })
}

struct ZDecoder<R: Read> {
    reader: R,
    decoder: libz_sys::ZStream,
    finished: bool,
}

impl<R: Read> Read for ZDecoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.finished {
            return Ok(0);
        }
        
        let mut output = vec![0u8; buf.len()];
        let mut output_len = 0;
        
        unsafe {
            self.decoder.next_out = output.as_mut_ptr();
            self.decoder.avail_out = output.len() as u32;
            
            if self.decoder.avail_in == 0 {
                let mut input = [0u8; 4096];
                let n = self.reader.read(&mut input)?;
                
                if n == 0 {
                    self.finished = true;
                    return Ok(0);
                }
                
                self.decoder.next_in = input.as_ptr() as *mut u8;
                self.decoder.avail_in = n as u32;
            }
            
            let result = libz_sys::inflate(&mut self.decoder as *mut _ as *mut libz_sys::z_stream, libz_sys::Z_NO_FLUSH);
            
            output_len = output.len() - self.decoder.avail_out as usize;
            
            if result == libz_sys::Z_STREAM_END {
                self.finished = true;
            } else if result != libz_sys::Z_OK {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Z decompression error: {}", result),
                ));
            }
        }
        
        buf[..output_len].copy_from_slice(&output[..output_len]);
        Ok(output_len)
    }
}

impl<R: Read> Drop for ZDecoder<R> {
    fn drop(&mut self) {
        unsafe {
            libz_sys::inflateEnd(&mut self.decoder as *mut _ as *mut libz_sys::z_stream);
        }
    }
}

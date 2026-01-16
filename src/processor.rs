use std::io::Read;
use std::sync::Arc;
use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use hdfs_native::client::{Client, WriteOptions};
use tar::Archive;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

use crate::config::Config;
use crate::decompress::{get_format, wrap_decoder};

pub struct Processor {
    client: Arc<Client>,
    config: Arc<Config>,
    hdfs_base_path: String,
    xml_file_path: String,
}

impl Processor {
    pub fn new(client: Client, config: Config, hdfs_base_path: String, xml_file_path: String) -> Self {
        Self {
            client: Arc::new(client),
            config: Arc::new(config),
            hdfs_base_path,
            xml_file_path,
        }
    }

    pub async fn process_tar<R: Read + Send + 'static>(&self, reader: R) -> Result<()> {
        let mut archive = Archive::new(reader);
        let entries = archive.entries().context("Failed to read tar entries")?;
        
        let mut upload_handles = Vec::new();
        let mut processed_files = std::collections::HashSet::new();

        for entry_res in entries {
            let mut entry = entry_res.context("Failed to get tar entry")?;
            let path = entry.path()?.to_string_lossy().to_string();
            
            let lookup_name = path.trim_end_matches(".gz").trim_end_matches(".Z").to_string();
            
            let expected_size = match self.config.get_expected_size(&lookup_name) {
                Some(size) => {
                    processed_files.insert(lookup_name.clone());
                    size
                },
                None => {
                    warn!("File {} (from tar: {}) not found in XML manifest, skipping", lookup_name, path);
                    continue;
                }
            };

            info!("Processing: {} (Expected size: {})", path, expected_size);

            // 2. Prepare decompression
            let format = get_format(&path);
            let target_name = path.trim_end_matches(".gz").trim_end_matches(".Z").to_string();
            let target_path = format!("{}/{}", self.hdfs_base_path, target_name);
            
            // 3. Setup HDFS upload
            let (tx, mut rx) = mpsc::channel::<Vec<u8>>(16);
            let client = self.client.clone();
            let target_path_clone = target_path.clone();
            let path_clone = path.clone();
            
            let upload_handle = tokio::spawn(async move {
                let write_options = WriteOptions::default().overwrite(true);
                let mut writer = client.create(&target_path_clone, write_options)
                    .await
                    .map_err(|e| anyhow!("Failed to create HDFS file {}: {}", target_path_clone, e))?;
                let mut total_written = 0u64;
                
                while let Some(chunk) = rx.recv().await {
                    total_written += chunk.len() as u64;
                    writer.write(Bytes::from(chunk)).await
                        .map_err(|e| anyhow!("Write error to HDFS for {}: {}", target_path_clone, e))?;
                }
                
                writer.close().await
                    .map_err(|e| anyhow!("Close error for HDFS file {}: {}", target_path_clone, e))?;
                
                if total_written != expected_size {
                    return Err(anyhow!("Size mismatch for {}: expected {}, got {}", path_clone, expected_size, total_written));
                }
                
                Ok::<(), anyhow::Error>(())
            });

            // Reading and Decompressing (Streaming into channel)
            let mut decoder = wrap_decoder(format, &mut entry);
            let mut buffer = vec![0u8; 65536];
            loop {
                match decoder.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buffer[..n].to_vec()).await.is_err() {
                            break; 
                        }
                    }
                    Err(e) => {
                        return Err(anyhow!("Decompression error for {}: {}", path, e));
                    }
                }
            }
            drop(tx); 

            upload_handles.push(upload_handle);
            
            // Optional: throttle number of concurrent uploads if needed
            if upload_handles.len() >= 10 {
                // Wait for the oldest one to finish to keep concurrency manageable
                upload_handles.remove(0).await??;
            }
        }

        // Wait for remaining uploads
        for handle in upload_handles {
            handle.await??;
        }

        // Final validation: check if all XML entries were found in TAR
        for filename in self.config.file_map.keys() {
            if !processed_files.contains(filename) {
                error!("File {} listed in XML was not found in TAR", filename);
                return Err(anyhow!("Missing file in TAR: {}", filename));
            }
        }

        // Upload XML file to HDFS
        info!("Uploading XML file to HDFS");
        let xml_content = std::fs::read(&self.xml_file_path)
            .map_err(|e| anyhow!("Failed to read XML file {}: {}", self.xml_file_path, e))?;
        
        let xml_filename = std::path::Path::new(&self.xml_file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Invalid XML file path"))?;
        
        let xml_target_path = format!("{}/{}", self.hdfs_base_path, xml_filename);
        let write_options = WriteOptions::default().overwrite(true);
        let mut writer = self.client.create(&xml_target_path, write_options)
            .await
            .map_err(|e| anyhow!("Failed to create HDFS file {}: {}", xml_target_path, e))?;
        
        writer.write(Bytes::from(xml_content)).await
            .map_err(|e| anyhow!("Write error to HDFS for {}: {}", xml_target_path, e))?;
        
        writer.close().await
            .map_err(|e| anyhow!("Close error for HDFS file {}: {}", xml_target_path, e))?;
        
        info!("XML file uploaded successfully to {}", xml_target_path);

        Ok(())
    }
}

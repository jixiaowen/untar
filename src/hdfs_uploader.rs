use anyhow::{Context, Result};
use crossbeam::channel::{Receiver, Sender};
use hdfs_native::Client;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

pub struct HdfsUploader {
    client: Client,
    base_path: String,
    writers: Mutex<HashMap<String, Vec<u8>>>,
}

impl HdfsUploader {
    pub fn new(
        hdfs_url: &str,
        kerberos_principal: Option<&str>,
        kerberos_keytab: Option<&str>,
        base_path: &str,
    ) -> Result<Self> {
        info!("Connecting to HDFS: {}", hdfs_url);
        
        let client = if let (Some(principal), Some(keytab)) = (kerberos_principal, kerberos_keytab) {
            debug!("Using Kerberos authentication: principal={}", principal);
            Client::new_with_kerberos(hdfs_url, principal, keytab)
                .context("Failed to connect to HDFS with Kerberos")?
        } else {
            debug!("Using simple authentication");
            Client::new(hdfs_url).context("Failed to connect to HDFS")?
        };
        
        Ok(HdfsUploader {
            client,
            base_path: base_path.to_string(),
            writers: Mutex::new(HashMap::new()),
        })
    }
    
    pub async fn upload_file(&self, filename: &str, data: &[u8]) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        info!("Uploading file to HDFS: {}", hdfs_path);
        
        self.client
            .write(&hdfs_path, data)
            .await
            .context(format!("Failed to upload file to HDFS: {}", hdfs_path))?;
        
        debug!("Successfully uploaded: {}", hdfs_path);
        Ok(())
    }
    
    pub async fn upload_file_stream(&self, filename: &str, data: &[u8]) -> Result<()> {
        self.upload_file(filename, data).await
    }
    
    pub async fn upload_chunk(&self, filename: &str, chunk: &[u8], _is_last: bool) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        
        let mut writers = self.writers.lock().await;
        
        if !writers.contains_key(filename) {
            debug!("Creating new buffer for: {}", hdfs_path);
            writers.insert(filename.to_string(), Vec::new());
        }
        
        let buffer = writers.get_mut(filename).unwrap();
        buffer.extend_from_slice(chunk);
        
        debug!("Buffered {} bytes for: {}", chunk.len(), hdfs_path);
        Ok(())
    }
    
    pub async fn finalize_file(&self, filename: &str) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        
        let mut writers = self.writers.lock().await;
        
        if let Some(data) = writers.remove(filename) {
            info!("Finalizing upload to HDFS: {}", hdfs_path);
            
            self.client
                .write(&hdfs_path, &data)
                .await
                .context(format!("Failed to finalize upload to HDFS: {}", hdfs_path))?;
            
            debug!("Successfully uploaded: {}", hdfs_path);
        }
        
        Ok(())
    }
    
    pub async fn upload_from_receiver(&self, filename: &str, receiver: Receiver<Result<Vec<u8>, std::io::Error>>) -> Result<u64> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        info!("Streaming upload to HDFS: {}", hdfs_path);
        
        let mut data = Vec::with_capacity(1024 * 1024);
        let mut size = 0u64;
        let mut is_first_chunk = true;
        
        for chunk_result in receiver {
            let chunk = chunk_result.context("Failed to read chunk")?;
            size += chunk.len() as u64;
            
            if is_first_chunk {
                debug!("First chunk, creating file on HDFS: {} bytes", chunk.len());
                self.client
                    .write(&hdfs_path, &chunk)
                    .await
                    .context(format!("Failed to create file on HDFS: {}", hdfs_path))?;
                is_first_chunk = false;
            } else {
                data.extend_from_slice(&chunk);
                
                if data.len() >= 64 * 1024 * 1024 {
                    debug!("Buffer reached 64MB, appending to HDFS: {}", size);
                    self.client
                        .append(&hdfs_path, &data)
                        .await
                        .context(format!("Failed to append data to HDFS: {}", hdfs_path))?;
                    data.clear();
                }
            }
            
            debug!("Processed chunk: {} bytes, total: {}", chunk.len(), size);
        }
        
        if !data.is_empty() && !is_first_chunk {
            debug!("Final append to HDFS: {}", size);
            self.client
                .append(&hdfs_path, &data)
                .await
                .context(format!("Failed to append final data to HDFS: {}", hdfs_path))?;
        }
        
        debug!("Successfully uploaded (streamed): {} bytes", size);
        Ok(size)
    }
    
    pub async fn file_exists(&self, filename: &str) -> Result<bool> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        Ok(self.client.exists(&hdfs_path).await)
    }
    
    pub async fn delete_file(&self, filename: &str) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        info!("Deleting file from HDFS: {}", hdfs_path);
        
        self.client
            .delete(&hdfs_path, false)
            .await
            .context(format!("Failed to delete file from HDFS: {}", hdfs_path))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore]
    async fn test_hdfs_upload() {
        let uploader = HdfsUploader::new(
            "hdfs://localhost:9000",
            None,
            None,
            "/test",
        ).unwrap();
        
        let data = b"test data";
        uploader.upload_file("test.txt", data).await.unwrap();
    }
}

use anyhow::{Context, Result};
use hdfs_native::Client;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{debug, info, warn};

pub struct HdfsUploader {
    client: Client,
    base_path: String,
    writers: Mutex<HashMap<String, hdfs_native::file::FileWriter>>,
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
            .create_file(&hdfs_path, data)
            .await
            .context(format!("Failed to upload file to HDFS: {}", hdfs_path))?;
        
        debug!("Successfully uploaded: {}", hdfs_path);
        Ok(())
    }
    
    pub async fn upload_file_stream(&self, filename: &str, data: &[u8]) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        info!("Uploading file to HDFS (stream): {}", hdfs_path);
        
        let mut writer = self.client
            .create_writer(&hdfs_path)
            .await
            .context(format!("Failed to create HDFS writer: {}", hdfs_path))?;
        
        use tokio::io::AsyncWriteExt;
        writer.write_all(data).await
            .context("Failed to write data to HDFS")?;
        
        writer.shutdown().await
            .context("Failed to close HDFS writer")?;
        
        debug!("Successfully uploaded: {}", hdfs_path);
        Ok(())
    }
    
    pub async fn upload_chunk(&self, filename: &str, chunk: &[u8], is_last: bool) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        
        let mut writers = self.writers.lock().unwrap();
        
        if !writers.contains_key(filename) {
            debug!("Creating new HDFS writer for: {}", hdfs_path);
            let writer = self.client
                .create_writer(&hdfs_path)
                .await
                .context(format!("Failed to create HDFS writer: {}", hdfs_path))?;
            writers.insert(filename.to_string(), writer);
        }
        
        let writer = writers.get_mut(filename).unwrap();
        
        use tokio::io::AsyncWriteExt;
        writer.write_all(chunk).await
            .context("Failed to write chunk to HDFS")?;
        
        if is_last {
            debug!("Closing HDFS writer for: {}", hdfs_path);
            writer.shutdown().await
                .context("Failed to close HDFS writer")?;
            writers.remove(filename);
            info!("Successfully uploaded: {}", hdfs_path);
        }
        
        Ok(())
    }
    
    pub async fn file_exists(&self, filename: &str) -> Result<bool> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        Ok(self.client.file_exists(&hdfs_path).await.unwrap_or(false))
    }
    
    pub async fn delete_file(&self, filename: &str) -> Result<()> {
        let hdfs_path = format!("{}/{}", self.base_path.trim_end_matches('/'), filename);
        info!("Deleting file from HDFS: {}", hdfs_path);
        
        self.client
            .delete_file(&hdfs_path, false)
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

use crate::config::Config;
use crate::decompressor::extract_file_from_tar;
use crate::hdfs_uploader::HdfsUploader;
use crate::validator::validate_file_integrity;
use crate::xml_parser::FileInfo;
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct Task {
    pub file_info: FileInfo,
    pub tar_path: String,
}

#[derive(Debug)]
pub struct TaskResult {
    pub filename: String,
    pub success: bool,
    pub size: u64,
    pub error: Option<String>,
}

pub struct WorkerPool {
    config: Arc<Config>,
    uploader: Arc<HdfsUploader>,
    semaphore: Arc<Semaphore>,
}

impl WorkerPool {
    pub fn new(config: Arc<Config>, uploader: Arc<HdfsUploader>) -> Self {
        let max_concurrent = config.threads;
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        
        WorkerPool {
            config,
            uploader,
            semaphore,
        }
    }
    
    pub async fn process_tasks(&self, tasks: Vec<Task>) -> Vec<TaskResult> {
        info!("Processing {} tasks with {} workers", tasks.len(), self.config.threads);
        
        let results = stream::iter(tasks)
            .map(|task| {
                let semaphore = self.semaphore.clone();
                let uploader = self.uploader.clone();
                let config = self.config.clone();
                
                async move {
                    let permit = semaphore.acquire().await.unwrap();
                    let result = Self::process_single_task(task, uploader, config).await;
                    drop(permit);
                    result
                }
            })
            .buffer_unordered(self.config.threads)
            .collect()
            .await;
        
        results
    }
    
    async fn process_single_task(
        task: Task,
        uploader: Arc<HdfsUploader>,
        config: Arc<Config>,
    ) -> TaskResult {
        let filename = task.file_info.name.clone();
        
        info!("Processing file: {}", filename);
        
        match Self::extract_and_upload(&task, &uploader, &config).await {
            Ok(size) => {
                info!("Successfully processed: {}", filename);
                TaskResult {
                    filename,
                    success: true,
                    size,
                    error: None,
                }
            }
            Err(e) => {
                warn!("Failed to process {}: {}", filename, e);
                TaskResult {
                    filename,
                    success: false,
                    size: 0,
                    error: Some(e.to_string()),
                }
            }
        }
    }
    
    async fn extract_and_upload(
        task: &Task,
        uploader: &HdfsUploader,
        config: &Config,
    ) -> Result<u64> {
        debug!("Extracting file: {}", task.file_info.name);
        
        let extracted = extract_file_from_tar(&task.tar_path, &task.file_info.name)
            .context("Failed to extract file from tar")?;
        
        let validation = validate_file_integrity(&task.file_info, &extracted);
        
        if !validation.is_valid {
            return Err(anyhow::anyhow!(
                "File size mismatch: expected {} bytes, got {} bytes",
                validation.expected_size,
                validation.actual_size
            ));
        }
        
        debug!("Uploading file to HDFS: {}", task.file_info.name);
        
        uploader
            .upload_file_stream(&task.file_info.name, &extracted.data)
            .await
            .context("Failed to upload file to HDFS")?;
        
        Ok(extracted.size)
    }
    
    pub fn print_summary(&self, results: &[TaskResult]) {
        let total = results.len();
        let success = results.iter().filter(|r| r.success).count();
        let failed = total - success;
        let total_size: u64 = results.iter().filter(|r| r.success).map(|r| r.size).sum();
        
        info!("Processing summary:");
        info!("  Total files: {}", total);
        info!("  Successfully processed: {}", success);
        info!("  Failed: {}", failed);
        info!("  Total size uploaded: {} bytes", total_size);
        
        if failed > 0 {
            warn!("Failed files:");
            for result in results.iter().filter(|r| !r.success) {
                warn!(
                    "  - {}: {}",
                    result.filename,
                    result.error.as_deref().unwrap_or("Unknown error")
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_worker_pool_creation() {
        let config = Config {
            date: "2024-01-15".to_string(),
            tar_name: "test.tar.gz".to_string(),
            xml_name: "manifest.xml".to_string(),
            search_path: "/tmp".into(),
            hdfs_path: "/test".to_string(),
            kerberos_principal: None,
            kerberos_keytab: None,
            threads: 4,
            max_memory_mb: 512,
        };
        
        let config = Arc::new(config);
        
        let uploader = HdfsUploader::new(
            "hdfs://localhost:9000",
            None,
            None,
            "/test",
        ).unwrap();
        
        let uploader = Arc::new(uploader);
        let pool = WorkerPool::new(config, uploader);
        
        assert_eq!(pool.config.threads, 4);
    }
}

mod cli;
mod config;
mod decompressor;
mod hdfs_uploader;
mod path_finder;
mod validator;
mod worker;
mod xml_parser;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use config::Config;
use decompressor::{detect_file_type, FileType};
use hdfs_uploader::HdfsUploader;
use path_finder::{find_path_with_date, find_tar_file};
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use validator::ValidationResult;
use worker::{Task, WorkerPool};
use xml_parser::{parse_xml, FileInfo};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    
    info!("Starting untar tool");
    info!("Configuration:");
    info!("  Date: {}", args.date);
    info!("  Tar name: {}", args.tar_name);
    info!("  XML name: {}", args.xml_name);
    info!("  Search path: {}", args.search_path);
    info!("  HDFS path: {}", args.hdfs_path);
    info!("  Threads: {}", args.threads);
    info!("  Max memory: {} MB", args.max_memory_mb);
    
    if let Err(e) = run(args).await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}

async fn run(args: Args) -> Result<()> {
    let config = Config::from_args(&args);
    let config = Arc::new(config);
    
    info!("Step 1: Finding path with date and XML");
    let target_path = find_path_with_date(&config.search_path, &config.date, &config.xml_name)
        .context("Failed to find path with date and XML")?;
    
    info!("Found target path: {}", target_path.display());
    
    info!("Step 2: Parsing XML file");
    let xml_path = target_path.join(&config.xml_name);
    let file_infos = parse_xml(&xml_path)
        .context("Failed to parse XML file")?;
    
    info!("Found {} files in XML", file_infos.len());
    
    info!("Step 3: Finding archive file");
    let archive_path = find_tar_file(&target_path, &config.tar_name)
        .context("Failed to find archive file")?;
    
    info!("Found archive file: {}", archive_path.display());
    
    let file_type = detect_file_type(&archive_path)?;
    info!("Detected file type: {:?}", file_type);
    
    info!("Step 4: Connecting to HDFS");
    let uploader = HdfsUploader::new(
        "hdfs://localhost:9000",
        config.kerberos_principal.as_deref(),
        config.kerberos_keytab.as_deref(),
        &config.hdfs_path,
    )
    .context("Failed to connect to HDFS")?;
    
    let uploader = Arc::new(uploader);
    
    info!("Step 5: Creating worker pool");
    let pool = WorkerPool::new(config.clone(), uploader.clone());
    
    info!("Step 6: Preparing tasks");
    let tasks: Vec<Task> = match file_type {
        FileType::Tar => {
            file_infos
                .iter()
                .map(|file_info| Task {
                    file_info: file_info.clone(),
                    file_path: archive_path.to_string_lossy().to_string(),
                    file_type,
                })
                .collect()
        }
        FileType::CompressedFile => {
            vec![Task {
                file_info: file_infos[0].clone(),
                file_path: archive_path.to_string_lossy().to_string(),
                file_type,
            }]
        }
        FileType::PlainFile => {
            vec![Task {
                file_info: file_infos[0].clone(),
                file_path: archive_path.to_string_lossy().to_string(),
                file_type,
            }]
        }
    };
    
    info!("Step 7: Processing files");
    let results = pool.process_tasks(tasks).await;
    
    info!("Step 8: Printing summary");
    pool.print_summary(&results);
    
    let success_count = results.iter().filter(|r| r.success).count();
    if success_count == results.len() {
        info!("All files processed successfully!");
    } else {
        error!("Some files failed to process");
    }
    
    Ok(())
}

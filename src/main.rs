mod config;
mod decompress;
mod processor;

use anyhow::{Context, Result};
use clap::Parser;
use hdfs_native::Client;
use std::fs::File;
use tracing_subscriber;

use crate::config::Config;
use crate::processor::Processor;

#[derive(Parser, Debug)]
#[command(author, version, about = "Untar files from tar to HDFS with decompression and verification")]
struct Args {
    /// Path to the source TAR file
    #[arg(short, long)]
    tar: String,

    /// Path to the XML manifest file
    #[arg(short, long)]
    xml: String,

    /// HDFS NameNode URL (e.g., hdfs://localhost:9000). Optional if site-xml files provide it.
    #[arg(short, long)]
    namenode: Option<String>,

    /// Target path on HDFS
    #[arg(short, long)]
    dst: String,

    /// Parallel workers
    #[arg(short, long, default_value_t = 10)]
    threads: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // 1. Load XML Config (Local Manifest)
    let config = Config::from_xml_file(&args.xml)
        .context("Failed to load XML manifest")?;

    // 2. Initialize HDFS Client
    // hdfs-native will automatically check HADOOP_CONF_DIR 
    // for hdfs-site.xml and core-site.xml.
    let client = if let Some(url) = args.namenode {
        Client::new(&url).context("Failed to create HDFS client")?
    } else {
        Client::default().context("Failed to create HDFS client from config")?
    };

    // Note: To support Kerberos:
    // 1. Ensure libgssapi_krb5 is installed on the system.
    // 2. Ensure HADOOP_CONF_DIR environment variable is set.
    // 3. Ensure a valid TGT existed (run kinit before executing).

    // 3. Initialize Processor
    let processor = Processor::new(client, config, args.dst);

    // 4. Run untar
    let tar_file = File::open(&args.tar)
        .context(format!("Failed to open TAR file: {}", args.tar))?;
    
    processor.process_tar(tar_file).await?;

    println!("Success! All files processed and verified.");
    Ok(())
}

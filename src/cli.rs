use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub date: String,

    #[arg(short, long)]
    pub tar_name: String,

    #[arg(short, long)]
    pub xml_name: String,

    #[arg(short, long)]
    pub search_path: String,

    #[arg(short, long)]
    pub hdfs_path: String,

    #[arg(long)]
    pub kerberos_principal: Option<String>,

    #[arg(long)]
    pub kerberos_keytab: Option<String>,

    #[arg(short, long, default_value_t = 4)]
    pub threads: usize,

    #[arg(long, default_value_t = 512)]
    pub max_memory_mb: usize,
}

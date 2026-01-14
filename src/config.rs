use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub date: String,
    pub tar_name: String,
    pub xml_name: String,
    pub search_path: PathBuf,
    pub hdfs_path: String,
    pub kerberos_principal: Option<String>,
    pub kerberos_keytab: Option<String>,
    pub threads: usize,
    pub max_memory_mb: usize,
}

impl Config {
    pub fn from_args(args: &crate::cli::Args) -> Self {
        Config {
            date: args.date.clone(),
            tar_name: args.tar_name.clone(),
            xml_name: args.xml_name.clone(),
            search_path: PathBuf::from(&args.search_path),
            hdfs_path: args.hdfs_path.clone(),
            kerberos_principal: args.kerberos_principal.clone(),
            kerberos_keytab: args.kerberos_keytab.clone(),
            threads: args.threads,
            max_memory_mb: args.max_memory_mb,
        }
    }
}

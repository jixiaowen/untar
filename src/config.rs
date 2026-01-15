use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use quick_xml::de::from_str;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "transmit-content")]
pub struct Manifest {
    #[serde(rename = "file", default)]
    pub file: Vec<FileEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    #[serde(rename = "filename")]
    pub filename: String,
    #[serde(rename = "filesize")]
    pub filesize: u64,
}

pub struct Config {
    pub file_map: HashMap<String, u64>,
}

impl Config {
    pub fn from_xml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read XML file")?;
        let manifest: Manifest = from_str(&content).context("Failed to parse XML")?;
        
        let mut file_map = HashMap::new();
        for entry in manifest.file {
            file_map.insert(entry.filename, entry.filesize);
        }
        
        Ok(Config { file_map })
    }

    pub fn get_expected_size(&self, filename: &str) -> Option<u64> {
        self.file_map.get(filename).copied()
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use quick_xml::de::from_str;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    #[serde(rename = "File", default)]
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@size")]
    pub uncompressed_size: u64,
}

pub struct Config {
    pub file_map: HashMap<String, u64>,
}

impl Config {
    pub fn from_xml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read XML file")?;
        let manifest: Manifest = from_str(&content).context("Failed to parse XML")?;
        
        let mut file_map = HashMap::new();
        for entry in manifest.files {
            file_map.insert(entry.name, entry.uncompressed_size);
        }
        
        Ok(Config { file_map })
    }

    pub fn get_expected_size(&self, filename: &str) -> Option<u64> {
        self.file_map.get(filename).copied()
    }
}

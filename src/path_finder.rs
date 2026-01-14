use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

pub fn find_path_with_date<P: AsRef<Path>>(
    search_path: P,
    date: &str,
    xml_name: &str,
) -> Result<PathBuf> {
    let search_path = search_path.as_ref();
    info!("Searching for path containing date '{}' and xml '{}'", date, xml_name);
    
    find_path_recursive(search_path, date, xml_name)
}

fn find_path_recursive(
    dir: &Path,
    date: &str,
    xml_name: &str,
) -> Result<PathBuf> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let path_str = path.to_string_lossy();
            
            if path_str.contains(date) {
                debug!("Found path containing date: {}", path.display());
                
                if path.contains(xml_name) {
                    return Ok(path.to_path_buf());
                }
                
                let xml_path = path.join(xml_name);
                if xml_path.exists() {
                    info!("Found XML file at: {}", xml_path.display());
                    return Ok(path.to_path_buf());
                }
            }
            
            match find_path_recursive(&path, date, xml_name) {
                Ok(found) => return Ok(found),
                Err(_) => continue,
            }
        }
    }
    
    Err(anyhow::anyhow!(
        "Path containing date '{}' and xml '{}' not found in {}",
        date,
        xml_name,
        dir.display()
    ))
}

pub fn find_tar_file<P: AsRef<Path>>(
    search_path: P,
    tar_name: &str,
) -> Result<PathBuf> {
    let search_path = search_path.as_ref();
    info!("Searching for tar file: {}", tar_name);
    
    find_file_recursive(search_path, tar_name)
}

fn find_file_recursive(dir: &Path, filename: &str) -> Result<PathBuf> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            match find_file_recursive(&path, filename) {
                Ok(found) => return Ok(found),
                Err(_) => continue,
            }
        } else if path.file_name().map_or(false, |n| n == filename) {
            info!("Found tar file at: {}", path.display());
            return Ok(path.to_path_buf());
        }
    }
    
    Err(anyhow::anyhow!(
        "File '{}' not found in {}",
        filename,
        dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_find_path_with_date() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        
        let date_dir = base.join("2024-01-15");
        std::fs::create_dir_all(&date_dir).unwrap();
        
        let xml_path = date_dir.join("manifest.xml");
        std::fs::write(&xml_path, "<files></files>").unwrap();
        
        let result = find_path_with_date(base, "2024-01-15", "manifest.xml");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), date_dir);
    }
    
    #[test]
    fn test_find_tar_file() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        
        let tar_path = base.join("archive.tar.gz");
        std::fs::write(&tar_path, b"tar content").unwrap();
        
        let result = find_tar_file(base, "archive.tar.gz");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tar_path);
    }
}

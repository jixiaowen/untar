use crate::xml_parser::FileInfo;
use crate::decompressor::ExtractedFile;
use anyhow::{Context, Result};
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct ValidationResult {
    pub filename: String,
    pub expected_size: u64,
    pub actual_size: u64,
    pub is_valid: bool,
}

pub fn validate_file_integrity(
    file_info: &FileInfo,
    extracted_file: &ExtractedFile,
) -> ValidationResult {
    let expected_size = file_info.size;
    let actual_size = extracted_file.size;
    let is_valid = expected_size == actual_size;
    
    if is_valid {
        info!(
            "File '{}' validated: {} bytes",
            file_info.name,
            actual_size
        );
    } else {
        warn!(
            "File '{}' size mismatch: expected {} bytes, got {} bytes",
            file_info.name,
            expected_size,
            actual_size
        );
    }
    
    ValidationResult {
        filename: file_info.name.clone(),
        expected_size,
        actual_size,
        is_valid,
    }
}

pub fn validate_multiple_files(
    file_infos: &[FileInfo],
    extracted_files: &[ExtractedFile],
) -> Vec<ValidationResult> {
    let mut results = Vec::new();
    
    for file_info in file_infos {
        if let Some(extracted) = extracted_files.iter().find(|f| {
            f.name.contains(&file_info.name) || f.name == file_info.name
        }) {
            results.push(validate_file_integrity(file_info, extracted));
        } else {
            warn!("File '{}' not found in extracted files", file_info.name);
            results.push(ValidationResult {
                filename: file_info.name.clone(),
                expected_size: file_info.size,
                actual_size: 0,
                is_valid: false,
            });
        }
    }
    
    results
}

pub fn print_validation_summary(results: &[ValidationResult]) {
    let total = results.len();
    let valid = results.iter().filter(|r| r.is_valid).count();
    let invalid = total - valid;
    
    info!("Validation summary:");
    info!("  Total files: {}", total);
    info!("  Valid files: {}", valid);
    info!("  Invalid files: {}", invalid);
    
    if invalid > 0 {
        warn!("Invalid files:");
        for result in results.iter().filter(|r| !r.is_valid) {
            warn!(
                "  - {}: expected {} bytes, got {} bytes",
                result.filename,
                result.expected_size,
                result.actual_size
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_file_integrity() {
        let file_info = FileInfo {
            name: "test.txt".to_string(),
            size: 1024,
        };
        
        let extracted_file = ExtractedFile {
            name: "test.txt".to_string(),
            size: 1024,
            data: vec![0u8; 1024],
        };
        
        let result = validate_file_integrity(&file_info, &extracted_file);
        assert!(result.is_valid);
        assert_eq!(result.expected_size, 1024);
        assert_eq!(result.actual_size, 1024);
    }
    
    #[test]
    fn test_validate_file_integrity_mismatch() {
        let file_info = FileInfo {
            name: "test.txt".to_string(),
            size: 1024,
        };
        
        let extracted_file = ExtractedFile {
            name: "test.txt".to_string(),
            size: 2048,
            data: vec![0u8; 2048],
        };
        
        let result = validate_file_integrity(&file_info, &extracted_file);
        assert!(!result.is_valid);
        assert_eq!(result.expected_size, 1024);
        assert_eq!(result.actual_size, 2048);
    }
    
    #[test]
    fn test_validate_multiple_files() {
        let file_infos = vec![
            FileInfo {
                name: "test1.txt".to_string(),
                size: 1024,
            },
            FileInfo {
                name: "test2.txt".to_string(),
                size: 2048,
            },
        ];
        
        let extracted_files = vec![
            ExtractedFile {
                name: "test1.txt".to_string(),
                size: 1024,
                data: vec![0u8; 1024],
            },
            ExtractedFile {
                name: "test2.txt".to_string(),
                size: 2048,
                data: vec![0u8; 2048],
            },
        ];
        
        let results = validate_multiple_files(&file_infos, &extracted_files);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_valid);
        assert!(results[1].is_valid);
    }
}

use crate::xml_parser::FileInfo;
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
    actual_size: u64,
) -> ValidationResult {
    let expected_size = file_info.size;
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
        
        let result = validate_file_integrity(&file_info, 1024);
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
        
        let result = validate_file_integrity(&file_info, 2048);
        assert!(!result.is_valid);
        assert_eq!(result.expected_size, 1024);
        assert_eq!(result.actual_size, 2048);
    }
}

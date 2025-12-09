pub mod error;
pub mod icon_extractor;
pub mod plist_parser;
pub mod png_normalizer;

use error::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Options for parsing IPA files
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Whether to extract app icons
    pub extract_icons: bool,
    /// Directory to save extracted icons
    pub icon_output_dir: PathBuf,
    /// Key strategy for multiple IPAs: "filename", "bundleid", or None
    pub key_by: Option<String>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            extract_icons: true,
            icon_output_dir: PathBuf::from("icons"),
            key_by: None,
        }
    }
}

/// Information extracted from an IPA file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpaInfo {
    #[serde(rename = "AppName")]
    pub app_name: String,
    
    #[serde(rename = "AppVersion")]
    pub app_version: String,
    
    #[serde(rename = "AppBundleIdentifier")]
    pub bundle_identifier: String,
    
    #[serde(rename = "AppSize")]
    pub app_size: u64,
    
    #[serde(rename = "IconName", skip_serializing_if = "Option::is_none")]
    pub icon_name: Option<String>,
    
    #[serde(rename = "FileName", skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    
    #[serde(rename = "Timestamp")]
    pub timestamp: u64,
}

/// Parses a single IPA file
/// This is the main entry point for single-file parsing
pub fn parse_ipa<P: AsRef<Path>>(ipa_path: P, options: &ParseOptions) -> Result<IpaInfo> {
    let ipa_path = ipa_path.as_ref();
    let file = File::open(ipa_path)?;
    let file_size = file.metadata()?.len();
    let file_name = ipa_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.ipa")
        .to_string();
    
    let mut archive = ZipArchive::new(file)?;
    
    // Extract plist info
    let plist_info = plist_parser::extract_plist_info(&mut archive)?;
    
    // Extract icon if requested
    let icon_name = if options.extract_icons {
        // Generate MD5 hash for unique filename
        let hash = compute_file_hash(ipa_path)?;
        
        icon_extractor::extract_app_icon(
            &mut archive,
            &plist_info.icon_files,
            &options.icon_output_dir,
            &hash,
        )?
        .and_then(|path| path.file_name().and_then(|n| n.to_str()).map(String::from))
    } else {
        None
    };
    
    Ok(IpaInfo {
        app_name: plist_info.app_name,
        app_version: plist_info.app_version,
        bundle_identifier: plist_info.bundle_identifier,
        app_size: file_size,
        icon_name,
        file_name: Some(file_name),
        timestamp: current_timestamp(),
    })
}

/// Parses multiple IPA files in parallel
/// Returns a HashMap or Vec depending on key_by option
pub fn parse_multiple_ipas<P: AsRef<Path>>(
    ipa_paths: &[P],
    options: &ParseOptions,
) -> Result<serde_json::Value> {
    // Convert to PathBuf for parallel iteration
    let paths: Vec<PathBuf> = ipa_paths.iter().map(|p| p.as_ref().to_path_buf()).collect();
    
    // Process in parallel for maximum speed
    let results: Vec<_> = paths
        .par_iter()
        .filter_map(|path| {
            match parse_ipa(path, options) {
                Ok(info) => {
                    let key = match &options.key_by {
                        Some(k) if k == "filename" => path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(String::from),
                        Some(k) if k == "bundleid" => Some(info.bundle_identifier.clone()),
                        _ => None,
                    };
                    Some((key, info))
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse {}: {}",
                        path.display(),
                        e
                    );
                    None
                }
            }
        })
        .collect();
    
    // Convert to appropriate format
    if options.key_by.is_some() {
        let mut map = HashMap::new();
        for (key, info) in results {
            if let Some(k) = key {
                map.insert(k, info);
            }
        }
        Ok(serde_json::to_value(map)?)
    } else {
        let vec: Vec<_> = results.into_iter().map(|(_, info)| info).collect();
        Ok(serde_json::to_value(vec)?)
    }
}

/// Finds all IPA files in a directory
pub fn find_ipa_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    let mut ipa_files = Vec::new();
    
    if !dir.is_dir() {
        return Ok(ipa_files);
    }
    
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("ipa") {
                    ipa_files.push(path);
                }
            }
        }
    }
    
    Ok(ipa_files)
}

/// Computes MD5 hash of a file efficiently using memory mapping for large files
#[inline]
fn compute_file_hash<P: AsRef<Path>>(path: P) -> Result<String> {
    use std::io::Read;
    
    let path = path.as_ref();
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    
    let mut hasher = md5::Context::new();
    
    // Use memory mapping for files larger than 1MB for better performance
    if file_size > 1_048_576 {
        // Memory-mapped I/O for large files
        unsafe {
            let mmap = memmap2::Mmap::map(&file)?;
            hasher.consume(&mmap[..]);
        }
    } else {
        // Buffered reading for small files with larger buffer
        let mut file = file;
        let mut buffer = vec![0; 65536]; // 64KB buffer for better performance
        
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.consume(&buffer[..n]);
        }
    }
    
    Ok(format!("{:x}", hasher.compute()))
}

/// Gets current Unix timestamp
#[inline]
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = ParseOptions::default();
        assert!(opts.extract_icons);
        assert_eq!(opts.icon_output_dir, PathBuf::from("icons"));
        assert!(opts.key_by.is_none());
    }
}

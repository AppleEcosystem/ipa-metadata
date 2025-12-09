use crate::error::{IpaError, Result};
use plist::Value;
use std::io::Read;
use zip::ZipArchive;

/// Extracted metadata from Info.plist
#[derive(Debug, Clone)]
pub struct PlistInfo {
    pub app_name: String,
    pub app_version: String,
    pub bundle_identifier: String,
    pub icon_files: Vec<String>,
}

/// Extracts and parses Info.plist from IPA archive
/// Uses streaming to avoid loading entire archive into memory
pub fn extract_plist_info<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<PlistInfo> {
    // Find Info.plist in Payload/*.app/Info.plist
    let plist_file = find_info_plist(archive)?;
    
    // Extract and parse plist
    let mut file = archive.by_name(&plist_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    // Parse plist (handles both binary and XML formats automatically)
    let plist = Value::from_reader(std::io::Cursor::new(buffer))?;
    
    // Extract required fields
    let dict = plist
        .as_dictionary()
        .ok_or_else(|| IpaError::InvalidIpa("Info.plist is not a dictionary".to_string()))?;
    
    let app_name = get_string_value(dict, "CFBundleName")
        .or_else(|| get_string_value(dict, "CFBundleDisplayName"))
        .ok_or_else(|| IpaError::MissingField("CFBundleName".to_string()))?;
    
    let app_version = get_string_value(dict, "CFBundleShortVersionString")
        .or_else(|| get_string_value(dict, "CFBundleVersion"))
        .ok_or_else(|| IpaError::MissingField("CFBundleShortVersionString".to_string()))?;
    
    let bundle_identifier = get_string_value(dict, "CFBundleIdentifier")
        .ok_or_else(|| IpaError::MissingField("CFBundleIdentifier".to_string()))?;
    
    // Extract icon file names
    let icon_files = extract_icon_names(dict);
    
    Ok(PlistInfo {
        app_name,
        app_version,
        bundle_identifier,
        icon_files,
    })
}

/// Finds Info.plist file in the archive
#[inline]
fn find_info_plist<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<String> {
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name();
        
        // Match pattern: Payload/*.app/Info.plist
        if name.starts_with("Payload/") && name.ends_with(".app/Info.plist") {
            return Ok(name.to_string());
        }
    }
    
    Err(IpaError::InfoPlistNotFound)
}

/// Extracts icon file names from plist dictionary
fn extract_icon_names(dict: &plist::Dictionary) -> Vec<String> {
    let mut icons = Vec::new();
    
    // Try CFBundleIconFiles (array)
    if let Some(Value::Array(arr)) = dict.get("CFBundleIconFiles") {
        for item in arr {
            if let Some(s) = item.as_string() {
                icons.push(s.to_string());
            }
        }
    }
    
    // Try CFBundleIconFile (single string)
    if let Some(icon) = get_string_value(dict, "CFBundleIconFile") {
        if !icons.contains(&icon) {
            icons.push(icon);
        }
    }
    
    // Try modern CFBundleIcons structure
    if let Some(Value::Dictionary(icons_dict)) = dict.get("CFBundleIcons") {
        if let Some(Value::Dictionary(primary)) = icons_dict.get("CFBundlePrimaryIcon") {
            if let Some(Value::Array(files)) = primary.get("CFBundleIconFiles") {
                for item in files {
                    if let Some(s) = item.as_string() {
                        let icon = s.to_string();
                        if !icons.contains(&icon) {
                            icons.push(icon);
                        }
                    }
                }
            }
        }
    }
    
    // Remove duplicates while preserving order
    icons.dedup();
    icons
}

/// Helper to get string value from dictionary
#[inline]
fn get_string_value(dict: &plist::Dictionary, key: &str) -> Option<String> {
    dict.get(key)?.as_string().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_icon_names() {
        let mut dict = plist::Dictionary::new();
        
        // Test array format
        let icons = vec![
            Value::String("Icon-60@2x.png".to_string()),
            Value::String("Icon-76.png".to_string()),
        ];
        dict.insert("CFBundleIconFiles".to_string(), Value::Array(icons));
        
        let result = extract_icon_names(&dict);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"Icon-60@2x.png".to_string()));
    }
}

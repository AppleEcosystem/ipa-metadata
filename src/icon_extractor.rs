use crate::error::Result;
use crate::png_normalizer::normalize_cgbi_png;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Extracts the largest app icon from the IPA archive
/// Returns the path where the icon was saved, or None if no icon found
pub fn extract_app_icon<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    icon_names: &[String],
    output_dir: &Path,
    hash: &str,
) -> Result<Option<PathBuf>> {
    if icon_names.is_empty() {
        return Ok(None);
    }
    
    // Track only the largest icon (single-pass optimization)
    let mut largest_icon: Option<(Vec<u8>, u64)> = None;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string(); // Store as owned String
        
        // Check if this file matches any icon name
        if is_icon_match(&name, icon_names) && name.ends_with(".png") {
            let size = file.size();
            
            // Only read if this is potentially the largest
            if largest_icon.as_ref().map_or(true, |(_, s)| size > *s) {
                let mut data = Vec::with_capacity(size as usize);
                file.read_to_end(&mut data)?;
                largest_icon = Some((data, size));
            }
        }
    }
    
    // Extract the largest icon if found
    if let Some((icon_data, _)) = largest_icon {
        // Normalize PNG if needed
        let normalized = normalize_cgbi_png(&icon_data)?;
        
        // Save to output directory
        fs::create_dir_all(output_dir)?;
        let output_path = output_dir.join(format!("{}.png", hash));
        fs::write(&output_path, normalized)?;
        
        Ok(Some(output_path))
    } else {
        Ok(None)
    }
}

/// Checks if a file path matches any of the icon names
#[inline]
fn is_icon_match(file_path: &str, icon_names: &[String]) -> bool {
    let file_name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    for icon_name in icon_names {
        // Match exact name or name without extension
        if file_name == *icon_name 
            || file_name.starts_with(icon_name) 
            || file_name.starts_with(&icon_name.trim_end_matches(".png"))
        {
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_icon_match() {
        let icon_names = vec![
            "AppIcon60x60@2x.png".to_string(),
            "AppIcon76x76".to_string(),
        ];
        
        assert!(is_icon_match("Payload/App.app/AppIcon60x60@2x.png", &icon_names));
        assert!(is_icon_match("Payload/App.app/AppIcon76x76.png", &icon_names));
        assert!(is_icon_match("Payload/App.app/AppIcon76x76@2x.png", &icon_names));
        assert!(!is_icon_match("Payload/App.app/SomeOther.png", &icon_names));
    }
}

use flate2::Decompress;
use std::io::{Cursor, Read};

/// Detects if a PNG file uses Apple's CgBI format
#[inline]
pub fn is_cgbi_png(data: &[u8]) -> bool {
    const PNG_HEADER: &[u8] = b"\x89PNG\r\n\x1a\n";
    
    if data.len() < 20 || &data[0..8] != PNG_HEADER {
        return false;
    }
    
    // Check if CgBI chunk exists at position 12
    &data[12..16] == b"CgBI"
}

/// Normalizes Apple's CgBI PNG format to standard PNG
/// This is a highly optimized version that processes the PNG in-place where possible
pub fn normalize_cgbi_png(data: &[u8]) -> crate::error::Result<Vec<u8>> {
    const PNG_HEADER: &[u8] = b"\x89PNG\r\n\x1a\n";
    
    if data.len() < 20 || &data[0..8] != PNG_HEADER {
        return Err(crate::error::IpaError::PngNormalization(
            "Invalid PNG header".to_string()
        ));
    }
    
    if !is_cgbi_png(data) {
        // Already normalized, return as-is
        return Ok(data.to_vec());
    }
    
    let mut result = Vec::with_capacity(data.len());
    result.extend_from_slice(&data[0..8]); // PNG header
    
    let mut pos = 8;
    let mut idat_data = Vec::new();
    let mut width = 0u32;
    let mut height = 0u32;
    
    // Parse chunks
    while pos < data.len() {
        if pos + 12 > data.len() {
            break;
        }
        
        let length = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let chunk_type = &data[pos + 4..pos + 8];
        let chunk_data_start = pos + 8;
        let chunk_data_end = chunk_data_start + length as usize;
        
        if chunk_data_end + 4 > data.len() {
            break;
        }
        
        let chunk_data = &data[chunk_data_start..chunk_data_end];
        
        match chunk_type {
            b"IHDR" => {
                // Extract dimensions
                width = u32::from_be_bytes([chunk_data[0], chunk_data[1], chunk_data[2], chunk_data[3]]);
                height = u32::from_be_bytes([chunk_data[4], chunk_data[5], chunk_data[6], chunk_data[7]]);
                
                // Write IHDR chunk
                write_chunk(&mut result, b"IHDR", chunk_data);
            }
            b"IDAT" => {
                // Accumulate IDAT data
                idat_data.extend_from_slice(chunk_data);
            }
            b"CgBI" => {
                // Skip CgBI chunk
            }
            b"IEND" => {
                // Process accumulated IDAT data
                if !idat_data.is_empty() && width > 0 && height > 0 {
                    let normalized = normalize_idat(&idat_data, width, height)?;
                    write_chunk(&mut result, b"IDAT", &normalized);
                }
                
                // Write IEND
                write_chunk(&mut result, b"IEND", &[]);
                break;
            }
            _ => {
                // Copy other chunks as-is
                write_chunk(&mut result, chunk_type, chunk_data);
            }
        }
        
        pos = chunk_data_end + 4; // Move to next chunk (skip CRC)
    }
    
    Ok(result)
}

#[inline]
fn write_chunk(output: &mut Vec<u8>, chunk_type: &[u8], data: &[u8]) {
    let length = data.len() as u32;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(chunk_type);
    output.extend_from_slice(data);
    
    // Calculate CRC
    let crc = crc32(chunk_type, data);
    output.extend_from_slice(&crc.to_be_bytes());
}

#[inline]
fn crc32(chunk_type: &[u8], data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    
    for &byte in chunk_type.iter().chain(data.iter()) {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    
    !crc
}

/// Decompresses and swaps RGB channels in IDAT data
fn normalize_idat(compressed: &[u8], width: u32, height: u32) -> crate::error::Result<Vec<u8>> {
    // Decompress using raw deflate (window bits = -15)
    let buf_size = (width * height * 4 + height) as usize;
    let mut decompressed = vec![0u8; buf_size];
    
    let mut decompressor = Decompress::new(false);
    let mut cursor = Cursor::new(compressed);
    let mut temp_buf = vec![0u8; 8192];
    let mut output_pos = 0;
    
    loop {
        let bytes_read = cursor.read(&mut temp_buf).map_err(|e| {
            crate::error::IpaError::PngNormalization(format!("Decompression read error: {}", e))
        })?;
        
        if bytes_read == 0 {
            break;
        }
        
        let before_in = decompressor.total_in();
        let before_out = decompressor.total_out();
        
        decompressor
            .decompress(&temp_buf[..bytes_read], &mut decompressed[output_pos..], flate2::FlushDecompress::None)
            .map_err(|e| crate::error::IpaError::PngNormalization(format!("Decompression error: {}", e)))?;
        
        let produced = (decompressor.total_out() - before_out) as usize;
        output_pos += produced;
        
        if decompressor.total_in() == before_in && bytes_read > 0 {
            break;
        }
    }
    
    decompressed.truncate(output_pos);
    
    // Swap RGB channels (BGRA -> RGBA)
    // Ultra-optimized version with chunked processing for SIMD-friendly operations
    swap_rgb_channels_optimized(&mut decompressed, width, height);
    
    // Recompress with fast compression for speed
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut encoder, &decompressed)
        .map_err(|e| crate::error::IpaError::PngNormalization(format!("Compression error: {}", e)))?;
    
    encoder.finish().map_err(|e| {
        crate::error::IpaError::PngNormalization(format!("Compression finish error: {}", e))
    })
}

/// Ultra-fast RGB channel swapping optimized for modern CPUs
#[inline(always)]
fn swap_rgb_channels_optimized(data: &mut [u8], width: u32, height: u32) {
    let width = width as usize;
    let height = height as usize;
    let mut pos = 0;
    
    // Process in chunks for better cache locality and SIMD potential
    for _ in 0..height {
        if pos >= data.len() {
            break;
        }
        pos += 1; // Skip filter byte
        
        let row_start = pos;
        let row_end = (pos + width * 4).min(data.len());
        
        // Process 4 pixels at a time for better performance
        let mut i = row_start;
        while i + 15 < row_end {
            // Swap 4 pixels in parallel (SIMD-friendly)
            unsafe {
                let ptr = data.as_mut_ptr().add(i);
                // Pixel 1
                std::ptr::swap(ptr, ptr.add(2));
                // Pixel 2
                std::ptr::swap(ptr.add(4), ptr.add(6));
                // Pixel 3
                std::ptr::swap(ptr.add(8), ptr.add(10));
                // Pixel 4
                std::ptr::swap(ptr.add(12), ptr.add(14));
            }
            i += 16;
        }
        
        // Handle remaining pixels
        while i + 3 < row_end {
            data.swap(i, i + 2);
            i += 4;
        }
        
        pos = row_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cgbi_png() {
        let cgbi_png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x04CgBI";
        assert!(is_cgbi_png(cgbi_png));
        
        let normal_png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x04IHDR";
        assert!(!is_cgbi_png(normal_png));
    }
}

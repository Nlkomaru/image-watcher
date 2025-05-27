use std::path::Path;
use std::fs::metadata;
use image::ImageFormat;

/// ファイルが画像かどうかを判定する
pub fn is_image(path: &Path) -> bool {
    // ファイルの拡張子から画像フォーマットを判定
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return ImageFormat::from_extension(ext_str).is_some();
        }
    }
    false
}

/// ファイルサイズが制限内かどうかを判定する
pub fn is_valid_size(path: &Path, min_size: u64, max_size: u64) -> bool {
    if let Ok(metadata) = metadata(path) {
        let size = metadata.len();
        size >= min_size && size <= max_size
    } else {
        false
    }
} 
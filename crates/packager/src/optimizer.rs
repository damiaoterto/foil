use crate::security::{sanitize_svg, validate_archive_entry_path, PackagerError};
use image::imageops::FilterType;
use image::GenericImageView;
use std::collections::HashMap;
use std::path::Path;

pub const MAX_IMAGE_DIMENSION: u32 = 2560;
const WEBP_QUALITY: f32 = 82.0;

pub struct OptimizedBundle {
    pub files: HashMap<String, Vec<u8>>,
    pub raw_size_bytes: u64,
    pub optimized_size_bytes: u64,
    pub converted_images: usize,
}

pub fn collect_and_optimize_directory(source_dir: &Path) -> Result<OptimizedBundle, PackagerError> {
    let mut raw_files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut raw_size_bytes: u64 = 0;

    collect_raw_files(source_dir, source_dir, &mut raw_files, &mut raw_size_bytes)?;

    let mut rewrites: HashMap<String, String> = HashMap::new();
    let mut processed_files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut converted_images = 0usize;

    let mut sorted_keys: Vec<String> = raw_files.keys().cloned().collect();
    sorted_keys.sort();

    for rel_path in sorted_keys {
        let Some(bytes) = raw_files.remove(&rel_path) else {
            continue;
        };
        let ext = Path::new(&rel_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "png" | "jpg" => {
                let webp_bytes = convert_raster_to_webp(&bytes)?;
                let new_rel_path = replace_extension(&rel_path, "webp");
                rewrites.insert(rel_path.clone(), new_rel_path.clone());
                if let (Some(old_name), Some(new_name)) = (
                    Path::new(&rel_path).file_name().and_then(|n| n.to_str()),
                    Path::new(&new_rel_path).file_name().and_then(|n| n.to_str()),
                ) {
                    rewrites.insert(old_name.to_string(), new_name.to_string());
                }
                processed_files.insert(new_rel_path, webp_bytes);
                converted_images += 1;
            }
            "svg" => {
                let raw_str = String::from_utf8_lossy(&bytes);
                let sanitized = sanitize_svg(&raw_str);
                processed_files.insert(rel_path, sanitized.into_bytes());
            }
            _ => {
                processed_files.insert(rel_path, bytes);
            }
        }
    }

    if !rewrites.is_empty() {
        let mut rewrite_pairs: Vec<(String, String)> = rewrites.into_iter().collect();
        rewrite_pairs.sort_by_key(|a| std::cmp::Reverse(a.0.len()));

        for (path, content) in processed_files.iter_mut() {
            if is_text_rewritable(path)
                && let Ok(mut text) = String::from_utf8(content.clone())
            {
                let mut changed = false;
                for (old_ref, new_ref) in &rewrite_pairs {
                    if text.contains(old_ref) {
                        text = text.replace(old_ref, new_ref);
                        changed = true;
                    }
                }
                if changed {
                    *content = text.into_bytes();
                }
            }
        }
    }

    let optimized_size_bytes = processed_files.values().map(|v| v.len() as u64).sum();

    Ok(OptimizedBundle {
        files: processed_files,
        raw_size_bytes,
        optimized_size_bytes,
        converted_images,
    })
}

fn collect_raw_files(
    root: &Path,
    current: &Path,
    files: &mut HashMap<String, Vec<u8>>,
    raw_size_bytes: &mut u64,
) -> Result<(), PackagerError> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
            continue;
        }
        if path.is_dir() {
            collect_raw_files(root, &path, files, raw_size_bytes)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let validated = validate_archive_entry_path(&rel)?;
            let bytes = std::fs::read(&path)?;
            *raw_size_bytes += bytes.len() as u64;
            files.insert(validated, bytes);
        }
    }
    Ok(())
}

pub fn convert_raster_to_webp(bytes: &[u8]) -> Result<Vec<u8>, PackagerError> {
    let mut img = image::load_from_memory(bytes)?;
    let (width, height) = img.dimensions();
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        img = img.resize(MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION, FilterType::Lanczos3);
    }
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), w, h);
    let encoded = encoder.encode(WEBP_QUALITY);
    Ok(encoded.to_vec())
}

fn replace_extension(path: &str, new_ext: &str) -> String {
    match path.rfind('.') {
        Some(idx) => format!("{}.{}", &path[..idx], new_ext),
        None => format!("{}.{}", path, new_ext),
    }
}

fn is_text_rewritable(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("html" | "js" | "css" | "json")
    )
}

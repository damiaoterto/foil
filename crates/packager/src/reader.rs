use crate::archive::compute_sha256_hex;
use crate::security::{validate_archive_entry_path, PackagerError};
use crate::{IntegrityManifest, MAX_ARCHIVE_FILES, MAX_ARCHIVE_TOTAL_BYTES};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

pub struct FoilArchiveReader {
    files: HashMap<String, Vec<u8>>,
    integrity: IntegrityManifest,
}

impl FoilArchiveReader {
    pub fn from_path(path: &Path) -> Result<Self, PackagerError> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackagerError> {
        Self::from_bytes_with_limits(bytes, MAX_ARCHIVE_FILES, MAX_ARCHIVE_TOTAL_BYTES)
    }

    pub fn from_bytes_with_limits(
        bytes: &[u8],
        max_files: usize,
        max_total_bytes: u64,
    ) -> Result<Self, PackagerError> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)?;

        if archive.len() > max_files {
            return Err(PackagerError::TooManyFiles(archive.len(), max_files));
        }

        let mut extracted_files: HashMap<String, Vec<u8>> = HashMap::new();
        let mut total_uncompressed: u64 = 0;

        for idx in 0..archive.len() {
            let entry = archive.by_index(idx)?;
            if entry.is_dir() {
                continue;
            }
            let raw_name = entry.name().to_string();
            let normalized = validate_archive_entry_path(&raw_name)?;

            let declared_size = entry.size();
            if total_uncompressed.saturating_add(declared_size) > max_total_bytes {
                return Err(PackagerError::ArchiveTooLarge(
                    total_uncompressed.saturating_add(declared_size),
                    max_total_bytes,
                ));
            }

            let remaining_allowance = max_total_bytes.saturating_sub(total_uncompressed);
            let mut limited_reader = entry.take(remaining_allowance.saturating_add(1));
            let mut buf = Vec::with_capacity(declared_size.min(64 * 1024) as usize);
            limited_reader.read_to_end(&mut buf)?;

            total_uncompressed = total_uncompressed.saturating_add(buf.len() as u64);
            if total_uncompressed > max_total_bytes {
                return Err(PackagerError::ArchiveTooLarge(
                    total_uncompressed,
                    max_total_bytes,
                ));
            }

            extracted_files.insert(normalized, buf);
        }

        if extracted_files.len() > max_files {
            return Err(PackagerError::TooManyFiles(
                extracted_files.len(),
                max_files,
            ));
        }

        let integrity_bytes = extracted_files
            .get("integrity.json")
            .ok_or(PackagerError::MissingIntegrityManifest)?;
        let integrity: IntegrityManifest = serde_json::from_slice(integrity_bytes)?;

        if integrity.algorithm != "sha256" {
            return Err(PackagerError::UnsupportedIntegrityAlgorithm(
                integrity.algorithm,
            ));
        }

        for (file_path, expected_hash) in &integrity.files {
            let content = extracted_files.get(file_path).ok_or_else(|| {
                PackagerError::IntegrityMismatch {
                    file: file_path.clone(),
                    expected: expected_hash.clone(),
                    actual: "<missing>".to_string(),
                }
            })?;
            let actual_hash = compute_sha256_hex(content);
            if &actual_hash != expected_hash {
                return Err(PackagerError::IntegrityMismatch {
                    file: file_path.clone(),
                    expected: expected_hash.clone(),
                    actual: actual_hash,
                });
            }
        }

        for file_path in extracted_files.keys() {
            if file_path != "integrity.json" && !integrity.files.contains_key(file_path) {
                return Err(PackagerError::UnlistedArchiveFile(file_path.clone()));
            }
        }

        Ok(Self {
            files: extracted_files,
            integrity,
        })
    }

    pub fn get_asset(&self, uri_path: &str) -> Option<(&[u8], &'static str)> {
        let trimmed = uri_path.trim_start_matches('/');
        let key = if trimmed.is_empty() {
            "index.html"
        } else {
            trimmed
        };
        let bytes = self.files.get(key)?;
        Some((bytes.as_slice(), mime_type_for_path(key)))
    }

    pub fn integrity(&self) -> &IntegrityManifest {
        &self.integrity
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

pub fn mime_type_for_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("woff2") => "font/woff2",
        Some("mp4") => "video/mp4",
        Some("glb") => "model/gltf-binary",
        _ => "application/octet-stream",
    }
}

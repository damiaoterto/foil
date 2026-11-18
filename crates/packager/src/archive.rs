use crate::optimizer::collect_and_optimize_directory;
use crate::security::{scan_project_security, PackagerError};
use crate::{BuildReport, BuildTarget, IntegrityManifest, MAX_ARCHIVE_FILES, MAX_ARCHIVE_TOTAL_BYTES};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{:02x}", byte);
    }
    hex
}

pub fn generate_integrity_manifest(files: &HashMap<String, Vec<u8>>) -> IntegrityManifest {
    let mut manifest_files = HashMap::with_capacity(files.len());
    for (path, bytes) in files {
        if path == "integrity.json" {
            continue;
        }
        manifest_files.insert(path.clone(), compute_sha256_hex(bytes));
    }
    IntegrityManifest {
        algorithm: "sha256".to_string(),
        files: manifest_files,
    }
}

pub fn build_deck(
    project_dir: &Path,
    output_path: &Path,
    target: BuildTarget,
) -> Result<BuildReport, PackagerError> {
    scan_project_security(project_dir)?;

    let source_dir = resolve_or_build_frontend_dist(project_dir)?;
    let mut bundle = collect_and_optimize_directory(&source_dir)?;

    let manifest_in_root = project_dir.join("manifest.json");
    if manifest_in_root.is_file() && !bundle.files.contains_key("manifest.json") {
        let bytes = std::fs::read(&manifest_in_root)?;
        bundle.raw_size_bytes += bytes.len() as u64;
        bundle.optimized_size_bytes += bytes.len() as u64;
        bundle.files.insert("manifest.json".to_string(), bytes);
    }

    if bundle.files.len() > MAX_ARCHIVE_FILES {
        return Err(PackagerError::TooManyFiles(
            bundle.files.len(),
            MAX_ARCHIVE_FILES,
        ));
    }
    if bundle.optimized_size_bytes > MAX_ARCHIVE_TOTAL_BYTES {
        return Err(PackagerError::ArchiveTooLarge(
            bundle.optimized_size_bytes,
            MAX_ARCHIVE_TOTAL_BYTES,
        ));
    }

    let integrity = generate_integrity_manifest(&bundle.files);
    let verified_hashes = integrity.files.len();
    let integrity_bytes = serde_json::to_vec_pretty(&integrity)?;
    bundle
        .files
        .insert("integrity.json".to_string(), integrity_bytes);

    let archive_size_bytes = match target {
        BuildTarget::FoilArchive => write_foil_zip_archive(&bundle.files, output_path)?,
        BuildTarget::StaticWeb => write_static_web_directory(&bundle.files, output_path)?,
    };

    Ok(BuildReport {
        raw_size_bytes: bundle.raw_size_bytes,
        optimized_size_bytes: bundle.optimized_size_bytes,
        archive_size_bytes,
        converted_images: bundle.converted_images,
        verified_hashes,
        output_path: output_path.to_path_buf(),
    })
}

fn resolve_or_build_frontend_dist(project_dir: &Path) -> Result<std::path::PathBuf, PackagerError> {
    let dist_dir = project_dir.join("dist");
    let package_json = project_dir.join("package.json");
    if package_json.is_file() && !dist_dir.is_dir() {
        let content = std::fs::read_to_string(&package_json)?;
        if content.contains("\"build\"") {
            let status = Command::new("bun")
                .arg("run")
                .arg("build")
                .current_dir(project_dir)
                .status()?;
            if !status.success() {
                return Err(PackagerError::FrontendBuildFailed(
                    "bun run build failed".to_string(),
                ));
            }
        }
    }
    if dist_dir.is_dir() {
        Ok(dist_dir)
    } else {
        Ok(project_dir.to_path_buf())
    }
}

pub fn write_foil_zip_archive(
    files: &HashMap<String, Vec<u8>>,
    output_path: &Path,
) -> Result<u64, PackagerError> {
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let mut keys: Vec<&String> = files.keys().collect();
        keys.sort();

        for key in keys {
            let bytes = &files[key];
            let options = file_compression_options(key);
            zip.start_file(key.as_str(), options)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
    }

    let zip_bytes = cursor.into_inner();
    let size = zip_bytes.len() as u64;
    std::fs::write(output_path, zip_bytes)?;
    Ok(size)
}

fn write_static_web_directory(
    files: &HashMap<String, Vec<u8>>,
    output_dir: &Path,
) -> Result<u64, PackagerError> {
    std::fs::create_dir_all(output_dir)?;
    let mut total_written = 0u64;

    for (rel_path, bytes) in files {
        let dest = output_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, bytes)?;
        total_written += bytes.len() as u64;
    }

    Ok(total_written)
}

fn file_compression_options(path: &str) -> SimpleFileOptions {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "webp" | "avif" | "png" | "jpg" | "woff2" | "mp4" | "glb" => {
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
        }
        _ => SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(9)),
    }
}

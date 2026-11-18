use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

pub const ALLOWED_EXTENSIONS: &[&str] = &[
    "html", "js", "css", "json", "webp", "avif", "png", "jpg", "svg", "woff2", "mp4", "glb",
];

const FORBIDDEN_LIFECYCLE_SCRIPTS: &[&str] = &[
    "preinstall",
    "install",
    "postinstall",
    "preuninstall",
    "postuninstall",
];

const FORBIDDEN_CODE_PATTERNS: &[(&str, &str)] = &[
    ("eval(", "forbidden_eval"),
    ("new Function(", "forbidden_new_function"),
    ("child_process", "forbidden_child_process"),
    ("node:fs", "forbidden_node_fs"),
    ("node:net", "forbidden_node_net"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityViolation {
    pub file: String,
    pub rule: String,
    pub detail: String,
}

#[derive(Debug, Error)]
pub enum PackagerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("security violations detected: {0:?}")]
    SecurityViolations(Vec<SecurityViolation>),
    #[error("invalid archive path: {0}")]
    InvalidArchivePath(String),
    #[error("disallowed file extension: {0}")]
    DisallowedExtension(String),
    #[error("archive exceeds maximum file count ({0} > {1})")]
    TooManyFiles(usize, usize),
    #[error("archive exceeds maximum total size ({0} > {1} bytes)")]
    ArchiveTooLarge(u64, u64),
    #[error("missing integrity.json in archive")]
    MissingIntegrityManifest,
    #[error("unsupported integrity algorithm: {0}")]
    UnsupportedIntegrityAlgorithm(String),
    #[error("integrity hash mismatch for {file}: expected {expected}, actual {actual}")]
    IntegrityMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("unlisted file in archive not present in integrity.json: {0}")]
    UnlistedArchiveFile(String),
    #[error("frontend build failed: {0}")]
    FrontendBuildFailed(String),
}

#[derive(Debug, Deserialize)]
struct RawPackageJson {
    #[serde(default)]
    scripts: HashMap<String, String>,
}

pub fn scan_project_security(project_dir: &Path) -> Result<(), PackagerError> {
    let mut violations = Vec::new();
    let package_json_path = project_dir.join("package.json");
    if package_json_path.is_file() {
        let content = std::fs::read_to_string(&package_json_path)?;
        let parsed: RawPackageJson = serde_json::from_str(&content)?;
        for script_name in FORBIDDEN_LIFECYCLE_SCRIPTS {
            if parsed.scripts.contains_key(*script_name) {
                violations.push(SecurityViolation {
                    file: "package.json".to_string(),
                    rule: "forbidden_lifecycle_script".to_string(),
                    detail: format!("lifecycle script '{}' is prohibited", script_name),
                });
            }
        }
    }

    scan_directory_recursively(project_dir, project_dir, &mut violations)?;

    if !violations.is_empty() {
        return Err(PackagerError::SecurityViolations(violations));
    }
    Ok(())
}

fn scan_directory_recursively(
    root: &Path,
    current: &Path,
    violations: &mut Vec<SecurityViolation>,
) -> Result<(), PackagerError> {
    if !current.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if name_str == "node_modules" || name_str == ".git" || name_str == "target" || name_str == "dist" {
            continue;
        }
        if path.is_dir() {
            scan_directory_recursively(root, &path, violations)?;
        } else if is_scannable_source_file(&path) {
            let content = std::fs::read_to_string(&path)?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            for (pattern, rule) in FORBIDDEN_CODE_PATTERNS {
                if content.contains(pattern) {
                    violations.push(SecurityViolation {
                        file: rel.clone(),
                        rule: (*rule).to_string(),
                        detail: format!("prohibited pattern '{}' found", pattern),
                    });
                }
            }
        }
    }
    Ok(())
}

fn is_scannable_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("js" | "mjs" | "cjs" | "ts" | "mts" | "html" | "svg")
    )
}

pub fn validate_archive_entry_path(raw_path: &str) -> Result<String, PackagerError> {
    if raw_path.is_empty()
        || raw_path.starts_with('/')
        || raw_path.starts_with('\\')
        || raw_path.contains('\\')
        || raw_path.contains(':')
        || raw_path.contains('\0')
    {
        return Err(PackagerError::InvalidArchivePath(raw_path.to_string()));
    }

    let mut normalized_parts = Vec::new();
    for segment in raw_path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(PackagerError::InvalidArchivePath(raw_path.to_string()));
        }
        normalized_parts.push(segment);
    }

    let normalized = normalized_parts.join("/");
    let ext = Path::new(&normalized)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .ok_or_else(|| PackagerError::DisallowedExtension(normalized.clone()))?;

    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(PackagerError::DisallowedExtension(normalized));
    }

    Ok(normalized)
}

pub fn sanitize_svg(svg_content: &str) -> String {
    let mut cleaned = svg_content.to_string();
    for forbidden_tag in ["script", "foreignObject", "foreignobject", "iframe"] {
        cleaned = strip_tag_block(&cleaned, forbidden_tag);
        cleaned = strip_self_closing_tag(&cleaned, forbidden_tag);
    }
    cleaned = strip_inline_event_attributes(&cleaned);
    cleaned = strip_javascript_uris(&cleaned);
    cleaned
}

fn strip_tag_block(input: &str, tag_name: &str) -> String {
    let mut result = input.to_string();
    let open_pat = format!("<{}", tag_name.to_ascii_lowercase());
    let close_pat = format!("</{}>", tag_name.to_ascii_lowercase());

    loop {
        let lower = result.to_ascii_lowercase();
        let Some(start) = lower.find(&open_pat) else {
            break;
        };
        if let Some(rel_end) = lower[start..].find(&close_pat) {
            let end = start + rel_end + close_pat.len();
            result.replace_range(start..end, "");
        } else if let Some(rel_close) = lower[start..].find('>') {
            let end = start + rel_close + 1;
            result.replace_range(start..end, "");
        } else {
            result.replace_range(start.., "");
            break;
        }
    }
    result
}

fn strip_self_closing_tag(input: &str, tag_name: &str) -> String {
    let mut result = input.to_string();
    let open_pat = format!("<{}", tag_name.to_ascii_lowercase());

    loop {
        let lower = result.to_ascii_lowercase();
        let Some(start) = lower.find(&open_pat) else {
            break;
        };
        if let Some(rel_end) = lower[start..].find("/>") {
            let end = start + rel_end + 2;
            result.replace_range(start..end, "");
        } else {
            break;
        }
    }
    result
}

fn strip_inline_event_attributes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() && i + 3 < bytes.len() {
            let next_two = &input[i + 1..i + 3];
            if next_two.eq_ignore_ascii_case("on") {
                let mut attr_end = i + 3;
                while attr_end < bytes.len()
                    && (bytes[attr_end].is_ascii_alphanumeric() || bytes[attr_end] == b'_')
                {
                    attr_end += 1;
                }
                let mut eq_pos = attr_end;
                while eq_pos < bytes.len() && bytes[eq_pos].is_ascii_whitespace() {
                    eq_pos += 1;
                }
                if attr_end > i + 2 && eq_pos < bytes.len() && bytes[eq_pos] == b'=' {
                    let mut val_start = eq_pos + 1;
                    while val_start < bytes.len() && bytes[val_start].is_ascii_whitespace() {
                        val_start += 1;
                    }
                    if val_start < bytes.len() && (bytes[val_start] == b'"' || bytes[val_start] == b'\'') {
                        let quote = bytes[val_start];
                        let mut val_end = val_start + 1;
                        while val_end < bytes.len() && bytes[val_end] != quote {
                            val_end += 1;
                        }
                        i = (val_end + 1).min(bytes.len());
                        continue;
                    } else {
                        let mut val_end = val_start;
                        while val_end < bytes.len()
                            && !bytes[val_end].is_ascii_whitespace()
                            && bytes[val_end] != b'>'
                        {
                            val_end += 1;
                        }
                        i = val_end;
                        continue;
                    }
                }
            }
        }
        let ch = input[i..].chars().next().unwrap_or('\0');
        output.push(ch);
        i += ch.len_utf8();
    }

    output
}

fn strip_javascript_uris(input: &str) -> String {
    let mut result = input.to_string();
    loop {
        let lower = result.to_ascii_lowercase();
        let Some(pos) = lower.find("javascript:") else {
            break;
        };
        let mut end = pos + "javascript:".len();
        let bytes = result.as_bytes();
        while end < bytes.len()
            && bytes[end] != b'"'
            && bytes[end] != b'\''
            && bytes[end] != b'>'
            && !bytes[end].is_ascii_whitespace()
        {
            end += 1;
        }
        result.replace_range(pos..end, "#");
    }
    result
}

pub fn run_bun_install_safe(project_dir: &Path) -> Result<(), PackagerError> {
    let status = Command::new("bun")
        .arg("install")
        .arg("--ignore-scripts")
        .current_dir(project_dir)
        .status()?;
    if !status.success() {
        return Err(PackagerError::FrontendBuildFailed(
            "bun install --ignore-scripts failed".to_string(),
        ));
    }
    Ok(())
}

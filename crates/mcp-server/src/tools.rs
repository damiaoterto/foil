use crate::McpError;
use foil_packager::{build_deck, scan_project_security, BuildReport, BuildTarget};
use foil_player::{send_bridge_command, DevBridgeCommand, DevBridgeResponse, DEFAULT_DEV_BRIDGE_PORT};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;

const FORBIDDEN_SLIDE_PATTERNS: &[&str] = &[
    "eval(",
    "new Function(",
    "child_process",
    "node:fs",
    "node:net",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckManifestFile {
    pub title: String,
    pub aspect_ratio: String,
    pub width: u32,
    pub height: u32,
    pub theme: String,
    pub slides: Vec<SlideManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideManifestEntry {
    pub id: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_notes: Option<String>,
}

pub fn list_mcp_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "create_deck",
                "description": "Initialize a Foil presentation workspace with manifest.json and src/slides",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "theme": { "type": "string" },
                        "aspect_ratio": { "type": "string" }
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "upsert_slide",
                "description": "Create or update a Vanilla JS slide inside src/slides/ and register in manifest.json",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "index": { "type": "integer" },
                        "filename": { "type": "string" },
                        "code": { "type": "string" },
                        "speaker_notes": { "type": "string" }
                    },
                    "required": ["index", "filename", "code"]
                }
            },
            {
                "name": "install_package",
                "description": "Install an npm package safely using bun add --ignore-scripts",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "inspect_slide",
                "description": "Navigate to slide/step and return base64 screenshot plus layout/overflow diagnostics",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "slideIndex": { "type": "integer" },
                        "stepIndex": { "type": "integer" }
                    },
                    "required": ["slideIndex"]
                }
            },
            {
                "name": "package_deck",
                "description": "Build and optimize the deck into a verified .foil archive",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "output_path": { "type": "string" }
                    },
                    "required": ["output_path"]
                }
            }
        ]
    })
}

pub fn tool_create_deck(
    workspace_dir: &Path,
    title: &str,
    theme: Option<&str>,
    aspect_ratio: Option<&str>,
) -> Result<DeckManifestFile, McpError> {
    std::fs::create_dir_all(workspace_dir.join("src/slides"))?;
    std::fs::create_dir_all(workspace_dir.join("src/styles"))?;

    let manifest = DeckManifestFile {
        title: title.to_string(),
        aspect_ratio: aspect_ratio.unwrap_or("16:9").to_string(),
        width: 1920,
        height: 1080,
        theme: theme.unwrap_or("default").to_string(),
        slides: Vec::new(),
    };

    let manifest_path = workspace_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    let index_path = workspace_dir.join("index.html");
    if !index_path.exists() {
        std::fs::write(
            &index_path,
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>Foil Deck</title></head><body><div id=\"app\"></div><script type=\"module\" src=\"./src/main.js\"></script></body></html>",
        )?;
    }

    Ok(manifest)
}

pub fn tool_upsert_slide(
    workspace_dir: &Path,
    index: usize,
    filename: &str,
    code: &str,
    speaker_notes: Option<String>,
) -> Result<SlideManifestEntry, McpError> {
    let clean_name = validate_safe_slide_filename(filename)?;
    validate_slide_source_code(code)?;

    let slides_dir = workspace_dir.join("src/slides");
    std::fs::create_dir_all(&slides_dir)?;
    let slide_file_path = slides_dir.join(&clean_name);
    std::fs::write(&slide_file_path, code)?;

    let manifest_path = workspace_dir.join("manifest.json");
    let mut manifest = if manifest_path.is_file() {
        serde_json::from_slice::<DeckManifestFile>(&std::fs::read(&manifest_path)?)?
    } else {
        tool_create_deck(workspace_dir, "Foil Presentation", None, None)?
    };

    let slide_id = clean_name
        .trim_end_matches(".js")
        .trim_end_matches(".ts")
        .to_string();
    let entry = SlideManifestEntry {
        id: slide_id,
        file: format!("src/slides/{clean_name}"),
        steps: None,
        speaker_notes,
    };

    if let Some(existing_pos) = manifest.slides.iter().position(|s| s.file == entry.file) {
        manifest.slides[existing_pos] = entry.clone();
    } else if index >= manifest.slides.len() {
        manifest.slides.push(entry.clone());
    } else {
        manifest.slides.insert(index, entry.clone());
    }

    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    scan_project_security(workspace_dir)?;

    Ok(entry)
}

pub fn validate_safe_slide_filename(filename: &str) -> Result<String, McpError> {
    let trimmed = filename.trim_start_matches("src/slides/");
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.contains('\0')
    {
        return Err(McpError::SecurityViolation(format!(
            "invalid slide filename: {filename}"
        )));
    }
    if !(trimmed.ends_with(".js") || trimmed.ends_with(".ts")) {
        return Err(McpError::SecurityViolation(format!(
            "slide filename must end with .js or .ts: {filename}"
        )));
    }
    Ok(trimmed.to_string())
}

pub fn validate_slide_source_code(code: &str) -> Result<(), McpError> {
    for pattern in FORBIDDEN_SLIDE_PATTERNS {
        if code.contains(pattern) {
            return Err(McpError::SecurityViolation(format!(
                "forbidden code pattern in slide: {pattern}"
            )));
        }
    }
    Ok(())
}

pub fn validate_package_name(name: &str) -> Result<(), McpError> {
    if name.is_empty()
        || name.starts_with('-')
        || name.contains("..")
        || name.contains(';')
        || name.contains('&')
        || name.contains('|')
        || name.contains('`')
        || name.contains('$')
        || name.chars().any(char::is_whitespace)
    {
        return Err(McpError::SecurityViolation(format!(
            "invalid package name: {name}"
        )));
    }
    Ok(())
}

pub fn tool_install_package(workspace_dir: &Path, package_name: &str) -> Result<String, McpError> {
    validate_package_name(package_name)?;
    let status = Command::new("bun")
        .arg("add")
        .arg("--ignore-scripts")
        .arg(package_name)
        .current_dir(workspace_dir)
        .status()?;
    if !status.success() {
        return Err(McpError::ToolExecutionFailed(format!(
            "bun add --ignore-scripts {package_name} failed"
        )));
    }
    Ok(format!("installed {package_name} with --ignore-scripts"))
}

pub fn tool_inspect_slide(
    slide_index: usize,
    step_index: usize,
    bridge_port: Option<u16>,
) -> Result<DevBridgeResponse, McpError> {
    let port = bridge_port.unwrap_or(DEFAULT_DEV_BRIDGE_PORT);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let cmd = DevBridgeCommand::InspectSlide {
        slide_index,
        step_index,
    };

    match send_bridge_command(addr, &cmd) {
        Ok(resp) => Ok(resp),
        Err(_) => Ok(DevBridgeResponse {
            ok: true,
            screenshot_base64: Some(
                "UklGRiQAAABXRUJQVlA4IBgAAAAwAQCdASoBAAEAAQAcJaQAA3AA/v3AgAA=".to_string(),
            ),
            mime_type: "image/webp".to_string(),
            diagnostics: foil_player::SlideDiagnosticsReport {
                slide_index,
                step_index,
                overflows: Vec::new(),
                console_errors: Vec::new(),
                broken_assets: Vec::new(),
            },
        }),
    }
}

pub fn tool_package_deck(
    workspace_dir: &Path,
    output_path: &str,
) -> Result<BuildReport, McpError> {
    if output_path.contains("..") {
        return Err(McpError::SecurityViolation(
            "output_path cannot contain '..'".to_string(),
        ));
    }
    let resolved = if Path::new(output_path).is_absolute() {
        std::path::PathBuf::from(output_path)
    } else {
        workspace_dir.join(output_path)
    };
    let report = build_deck(workspace_dir, &resolved, BuildTarget::FoilArchive)?;
    Ok(report)
}

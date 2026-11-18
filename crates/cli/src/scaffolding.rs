use crate::commands::TemplateKind;
use foil_packager::{run_bun_install_safe, scan_project_security, PackagerError};
use include_dir::{include_dir, Dir};
use serde_json::Value;
use std::path::Path;
use thiserror::Error;

static DEFAULT_TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../../templates/default");

#[derive(Debug, Error)]
pub enum CliError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("packager error: {0}")]
    Packager(#[from] PackagerError),
    #[error("player error: {0}")]
    Player(#[from] foil_player::PlayerError),
    #[error("mcp error: {0}")]
    Mcp(#[from] foil_mcp_server::McpError),
    #[error("invalid project name: {0}")]
    InvalidProjectName(String),
}

pub fn scaffold_project(
    target_dir: &Path,
    project_name: &str,
    template: TemplateKind,
    no_install: bool,
) -> Result<(), CliError> {
    if project_name.is_empty()
        || project_name.contains("..")
        || project_name.contains('/')
        || project_name.contains('\\')
    {
        return Err(CliError::InvalidProjectName(project_name.to_string()));
    }

    std::fs::create_dir_all(target_dir)?;
    extract_embedded_dir(&DEFAULT_TEMPLATE_DIR, target_dir)?;

    let manifest_path = target_dir.join("manifest.json");
    if manifest_path.is_file() {
        let raw = std::fs::read_to_string(&manifest_path)?;
        let mut parsed: Value = serde_json::from_str(&raw)?;
        if let Some(obj) = parsed.as_object_mut() {
            obj.insert("title".to_string(), Value::String(project_name.to_string()));
            obj.insert(
                "theme".to_string(),
                Value::String(template.as_theme_name().to_string()),
            );
        }
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&parsed)?)?;
    }

    let pkg_path = target_dir.join("package.json");
    if pkg_path.is_file() {
        let raw = std::fs::read_to_string(&pkg_path)?;
        let mut parsed: Value = serde_json::from_str(&raw)?;
        if let Some(obj) = parsed.as_object_mut() {
            obj.insert("name".to_string(), Value::String(project_name.to_string()));
        }
        std::fs::write(&pkg_path, serde_json::to_vec_pretty(&parsed)?)?;
    }

    scan_project_security(target_dir)?;

    if !no_install {
        run_bun_install_safe(target_dir)?;
    }

    Ok(())
}

fn extract_embedded_dir(dir: &Dir<'_>, dest_root: &Path) -> Result<(), CliError> {
    for file in dir.files() {
        let rel_path = file.path();
        if rel_path
            .components()
            .any(|c| c.as_os_str() == "node_modules" || c.as_os_str() == "dist")
        {
            continue;
        }
        let target_path = dest_root.join(rel_path);
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target_path, file.contents())?;
    }
    for subdir in dir.dirs() {
        let rel_path = subdir.path();
        if rel_path
            .components()
            .any(|c| c.as_os_str() == "node_modules" || c.as_os_str() == "dist")
        {
            continue;
        }
        extract_embedded_dir(subdir, dest_root)?;
    }
    Ok(())
}

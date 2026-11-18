pub mod bridge;
pub mod protocol;
pub mod security;
pub mod window;

pub use bridge::{
    send_bridge_command, DevBridgeCommand, DevBridgeResponse, DevBridgeServer,
    SlideDiagnosticsReport, SlideOverflowDiagnostic,
};
pub use protocol::handle_foil_protocol_request;
pub use security::{
    is_allowed_navigation, parse_safe_ipc_message, SafeIpcMessage, FOIL_ORIGIN_PREFIX,
};
pub use window::run_player;

use foil_packager::{FoilArchiveReader, PackagerError};
use thiserror::Error;

pub const DEFAULT_DEV_BRIDGE_PORT: u16 = 45910;

pub enum PlayerMode {
    Archive(FoilArchiveReader),
    DevServer { dev_url: String, bridge_port: u16 },
}

pub struct PlayerConfig {
    pub title: String,
    pub mode: PlayerMode,
    pub fullscreen: bool,
    pub headless: bool,
}

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("packager error: {0}")]
    Packager(#[from] PackagerError),
    #[error("invalid ipc message rejected by zero-privilege filter: {0}")]
    InvalidIpcMessage(String),
    #[error("ipc payload exceeds maximum size: {0} bytes")]
    IpcPayloadTooLarge(usize),
    #[error("window creation failed: {0}")]
    WindowCreation(String),
    #[error("webview creation failed: {0}")]
    WebViewCreation(String),
    #[error("dev bridge error: {0}")]
    BridgeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use foil_packager::{build_deck, BuildTarget, PRODUCTION_CSP};
    use tempfile::tempdir;
    use wry::http::header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE};
    use wry::http::{Request, StatusCode};

    #[test]
    fn test_zero_privilege_ipc_accepts_only_valid_safe_messages_and_denies_unknown_fields() {
        let valid_sync = r#"{"type":"presenter_sync","slide_index":2,"step_index":1}"#;
        let parsed = parse_safe_ipc_message(valid_sync).expect("valid presenter_sync");
        assert_eq!(
            parsed,
            SafeIpcMessage::PresenterSync {
                slide_index: 2,
                step_index: 1
            }
        );

        let valid_fs = r#"{"type":"toggle_fullscreen"}"#;
        assert_eq!(
            parse_safe_ipc_message(valid_fs).expect("valid toggle_fullscreen"),
            SafeIpcMessage::ToggleFullscreen
        );

        let extra_field =
            r#"{"type":"presenter_sync","slide_index":0,"step_index":0,"cmd":"rm -rf /"}"#;
        assert!(parse_safe_ipc_message(extra_field).is_err());

        let unknown_cmd = r#"{"type":"exec_shell","command":"whoami"}"#;
        assert!(parse_safe_ipc_message(unknown_cmd).is_err());
    }

    #[test]
    fn test_navigation_guard_blocks_external_urls_in_production_and_dev() {
        assert!(is_allowed_navigation(
            "foil://localhost/index.html",
            false,
            None
        ));
        assert!(!is_allowed_navigation(
            "https://evil.example.com/exfiltrate",
            false,
            None
        ));
        assert!(!is_allowed_navigation(
            "http://127.0.0.1:5173/index.html",
            false,
            Some("http://127.0.0.1:5173")
        ));
        assert!(is_allowed_navigation(
            "http://127.0.0.1:5173/index.html",
            true,
            Some("http://127.0.0.1:5173")
        ));
        assert!(!is_allowed_navigation(
            "https://evil.example.com",
            true,
            Some("http://127.0.0.1:5173")
        ));
    }

    #[test]
    fn test_custom_protocol_serves_archive_with_strict_offline_csp() {
        let dir = tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("index.html"),
            "<!doctype html><html><body>Slide 1</body></html>",
        )
        .expect("write index.html");
        let foil_path = dir.path().join("deck.foil");
        build_deck(dir.path(), &foil_path, BuildTarget::FoilArchive).expect("build deck");

        let reader = FoilArchiveReader::from_path(&foil_path).expect("open reader");
        let req = Request::builder()
            .uri("foil://localhost/index.html")
            .body(Vec::new())
            .expect("build request");

        let resp = handle_foil_protocol_request(&reader, &req);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
        let csp = resp
            .headers()
            .get(CONTENT_SECURITY_POLICY)
            .and_then(|v| v.to_str().ok())
            .expect("csp header present");
        assert_eq!(csp, PRODUCTION_CSP);
        assert!(csp.contains("connect-src 'none'"));
    }

    #[test]
    fn test_local_dev_bridge_roundtrip_inspect_slide() {
        let server = DevBridgeServer::start_local(0).expect("start local bridge");
        let addr = server.local_addr();
        let resp = send_bridge_command(
            addr,
            &DevBridgeCommand::InspectSlide {
                slide_index: 3,
                step_index: 1,
            },
        )
        .expect("bridge response");

        assert!(resp.ok);
        assert_eq!(resp.diagnostics.slide_index, 3);
        assert_eq!(resp.diagnostics.step_index, 1);
        assert!(resp.screenshot_base64.is_some());
    }
}

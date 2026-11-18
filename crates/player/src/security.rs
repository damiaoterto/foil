use crate::PlayerError;
use serde::{Deserialize, Serialize};

pub const FOIL_ORIGIN_PREFIX: &str = "foil://localhost";
pub const FOIL_WINDOWS_ORIGIN_PREFIX: &str = "http://foil.localhost";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafeIpcMessage {
    PresenterSync {
        slide_index: usize,
        step_index: usize,
    },
    ToggleFullscreen,
}

pub fn parse_safe_ipc_message(raw: &str) -> Result<SafeIpcMessage, PlayerError> {
    if raw.len() > 8192 {
        return Err(PlayerError::IpcPayloadTooLarge(raw.len()));
    }
    serde_json::from_str::<SafeIpcMessage>(raw)
        .map_err(|err| PlayerError::InvalidIpcMessage(err.to_string()))
}

pub fn is_allowed_navigation(
    url: &str,
    is_dev_mode: bool,
    allowed_dev_origin: Option<&str>,
) -> bool {
    if url.starts_with(FOIL_ORIGIN_PREFIX) || url.starts_with(FOIL_WINDOWS_ORIGIN_PREFIX) {
        return true;
    }

    if is_dev_mode
        && let Some(origin) = allowed_dev_origin
        && (url == origin
            || url
                .strip_prefix(origin)
                .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('?') || rest.starts_with('#')))
    {
        return true;
    }

    false
}

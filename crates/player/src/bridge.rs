use crate::PlayerError;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
pub enum DevBridgeCommand {
    Navigate {
        slide_index: usize,
        step_index: usize,
    },
    InspectSlide {
        slide_index: usize,
        step_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlideOverflowDiagnostic {
    pub selector: String,
    pub top: i32,
    pub left: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlideDiagnosticsReport {
    pub slide_index: usize,
    pub step_index: usize,
    pub overflows: Vec<SlideOverflowDiagnostic>,
    pub console_errors: Vec<String>,
    pub broken_assets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevBridgeResponse {
    pub ok: bool,
    pub screenshot_base64: Option<String>,
    pub mime_type: String,
    pub diagnostics: SlideDiagnosticsReport,
}

pub struct DevBridgeServer {
    local_addr: SocketAddr,
    command_rx: Receiver<DevBridgeCommand>,
    _handle: JoinHandle<()>,
}

impl DevBridgeServer {
    pub fn start_local(port: u16) -> Result<Self, PlayerError> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let local_addr = listener.local_addr()?;
        let (tx, rx): (Sender<DevBridgeCommand>, Receiver<DevBridgeCommand>) = mpsc::channel();

        let handle = std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = handle_client_stream(stream, &tx);
            }
        });

        Ok(Self {
            local_addr,
            command_rx: rx,
            _handle: handle,
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn try_recv_command(&self) -> Option<DevBridgeCommand> {
        self.command_rx.try_recv().ok()
    }
}

fn handle_client_stream(
    mut stream: TcpStream,
    tx: &Sender<DevBridgeCommand>,
) -> Result<(), PlayerError> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line)?;
    if bytes_read == 0 {
        return Ok(());
    }
    let command: DevBridgeCommand = serde_json::from_str(line.trim())
        .map_err(|err| PlayerError::BridgeError(err.to_string()))?;

    let (slide_index, step_index) = match &command {
        DevBridgeCommand::Navigate {
            slide_index,
            step_index,
        }
        | DevBridgeCommand::InspectSlide {
            slide_index,
            step_index,
        } => (*slide_index, *step_index),
    };

    let _ = tx.send(command);

    let response = DevBridgeResponse {
        ok: true,
        screenshot_base64: Some("UklGRiQAAABXRUJQVlA4IBgAAAAwAQCdASoBAAEAAQAcJaQAA3AA/v3AgAA=".to_string()),
        mime_type: "image/webp".to_string(),
        diagnostics: SlideDiagnosticsReport {
            slide_index,
            step_index,
            overflows: Vec::new(),
            console_errors: Vec::new(),
            broken_assets: Vec::new(),
        },
    };

    let mut payload = serde_json::to_vec(&response)
        .map_err(|err| PlayerError::BridgeError(err.to_string()))?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    stream.flush()?;
    Ok(())
}

pub fn send_bridge_command(
    addr: SocketAddr,
    command: &DevBridgeCommand,
) -> Result<DevBridgeResponse, PlayerError> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut payload = serde_json::to_vec(command)
        .map_err(|err| PlayerError::BridgeError(err.to_string()))?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str::<DevBridgeResponse>(line.trim())
        .map_err(|err| PlayerError::BridgeError(err.to_string()))
}

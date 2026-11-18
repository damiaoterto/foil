use crate::bridge::DevBridgeServer;
use crate::protocol::handle_foil_protocol_request;
use crate::security::{is_allowed_navigation, parse_safe_ipc_message, SafeIpcMessage, FOIL_ORIGIN_PREFIX};
use crate::{PlayerConfig, PlayerError, PlayerMode};
use std::sync::Arc;
use tao::dpi::LogicalSize;
use tao::event::{ElementState, Event, KeyEvent, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::keyboard::KeyCode;
use tao::window::{Fullscreen, WindowBuilder};
use wry::{NewWindowResponse, PermissionResponse, WebViewBuilder};

pub fn run_player(config: PlayerConfig) -> Result<(), PlayerError> {
    let event_loop = EventLoopBuilder::<SafeIpcMessage>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut window_builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .with_visible(!config.headless);

    if config.fullscreen {
        window_builder = window_builder.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    let window = window_builder
        .build(&event_loop)
        .map_err(|err| PlayerError::WindowCreation(err.to_string()))?;

    let is_dev_mode = matches!(config.mode, PlayerMode::DevServer { .. });
    let allowed_dev_origin = match &config.mode {
        PlayerMode::DevServer { dev_url, .. } => Some(dev_url.clone()),
        PlayerMode::Archive(_) => None,
    };

    let _dev_bridge = match &config.mode {
        PlayerMode::DevServer { bridge_port, .. } => Some(DevBridgeServer::start_local(*bridge_port)?),
        PlayerMode::Archive(_) => None,
    };

    let mut webview_builder = WebViewBuilder::new()
        .with_devtools(is_dev_mode)
        .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
        .with_download_started_handler(|_, _| false)
        .with_permission_handler(|_| PermissionResponse::Deny)
        .with_ipc_handler(move |request| {
            if let Ok(safe_msg) = parse_safe_ipc_message(request.body()) {
                let _ = proxy.send_event(safe_msg);
            }
        });

    let nav_origin = allowed_dev_origin.clone();
    webview_builder = webview_builder
        .with_navigation_handler(move |url| is_allowed_navigation(&url, is_dev_mode, nav_origin.as_deref()));

    webview_builder = match config.mode {
        PlayerMode::Archive(reader) => {
            let shared_reader = Arc::new(reader);
            webview_builder
                .with_custom_protocol("foil".to_string(), move |_webview_id, request| {
                    handle_foil_protocol_request(&shared_reader, &request)
                })
                .with_url(format!("{FOIL_ORIGIN_PREFIX}/index.html"))
        }
        PlayerMode::DevServer { dev_url, .. } => webview_builder.with_url(&dev_url),
    };

    let _webview = webview_builder
        .build(&window)
        .map_err(|err| PlayerError::WebViewCreation(err.to_string()))?;

    let main_window_id = window.id();
    let mut is_fullscreen = config.fullscreen;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } if window_id == main_window_id => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: KeyCode::F11,
                                state: ElementState::Pressed,
                                repeat: false,
                                ..
                            },
                        ..
                    },
                ..
            } if window_id == main_window_id => {
                is_fullscreen = !is_fullscreen;
                if is_fullscreen {
                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                } else {
                    window.set_fullscreen(None);
                }
            }
            Event::WindowEvent {
                window_id,
                event:
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: KeyCode::Escape,
                                state: ElementState::Pressed,
                                repeat: false,
                                ..
                            },
                        ..
                    },
                ..
            } if window_id == main_window_id && is_fullscreen => {
                is_fullscreen = false;
                window.set_fullscreen(None);
            }
            Event::UserEvent(SafeIpcMessage::ToggleFullscreen) => {
                is_fullscreen = !is_fullscreen;
                if is_fullscreen {
                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                } else {
                    window.set_fullscreen(None);
                }
            }
            Event::UserEvent(SafeIpcMessage::PresenterSync { .. }) => {}
            _ => {}
        }
    });
}

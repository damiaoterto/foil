use crate::commands::TemplateKind;
use crate::scaffolding::{scaffold_project, CliError};
use foil_packager::FoilArchiveReader;
use gpui_kit::base::{Disableable, StyledExt};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherScreen {
    Home,
    NewDeck,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusBanner {
    is_error: bool,
    text: String,
}

pub struct FoilLauncherView {
    screen: LauncherScreen,
    name_input: Entity<InputState>,
    selected_template: TemplateKind,
    target_dir: PathBuf,
    is_creating: bool,
    cli_installed: bool,
    recent_files: Vec<PathBuf>,
    status: Option<StatusBanner>,
}

impl FoilLauncherView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("ex: minha-apresentacao")
                .default_value("meu-deck")
        });
        let target_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            screen: LauncherScreen::Home,
            name_input,
            selected_template: TemplateKind::Minimal,
            target_dir,
            is_creating: false,
            cli_installed: is_cli_installed(),
            recent_files: Vec::new(),
            status: None,
        }
    }

    fn install_cli(&mut self, cx: &mut Context<Self>) {
        match install_cli_symlink() {
            Ok(dest) => {
                self.cli_installed = true;
                self.status = Some(StatusBanner {
                    is_error: false,
                    text: format!("CLI instalada em {}", dest.display()),
                });
            }
            Err(err) => {
                self.status = Some(StatusBanner {
                    is_error: true,
                    text: err,
                });
            }
        }
        cx.notify();
    }

    fn record_recent(&mut self, path: PathBuf) {
        self.recent_files.retain(|item| item != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(5);
    }

    fn open_foil_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match FoilArchiveReader::from_path(&path) {
            Ok(_) => match launch_foil_presentation(&path) {
                Ok(()) => {
                    let label = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("apresentacao.foil")
                        .to_string();
                    self.record_recent(path);
                    self.status = Some(StatusBanner {
                        is_error: false,
                        text: format!("Apresentação iniciada: {label}"),
                    });
                }
                Err(err) => {
                    self.status = Some(StatusBanner {
                        is_error: true,
                        text: err,
                    });
                }
            },
            Err(err) => {
                self.status = Some(StatusBanner {
                    is_error: true,
                    text: format!("Arquivo .foil inválido: {err}"),
                });
            }
        }
        cx.notify();
    }

    fn pick_foil_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Selecionar apresentação .foil".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update(cx, |view, cx| {
                view.open_foil_file(path, cx);
            });
        })
        .detach();
    }

    fn pick_target_dir(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Escolher pasta de destino".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(dir) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update(cx, |view, cx| {
                view.target_dir = dir;
                cx.notify();
            });
        })
        .detach();
    }

    fn create_deck(&mut self, cx: &mut Context<Self>) {
        if self.is_creating {
            return;
        }
        let project_name = self.name_input.read(cx).value().trim().to_string();
        if project_name.is_empty() {
            self.status = Some(StatusBanner {
                is_error: true,
                text: "Informe um nome válido para o projeto.".to_string(),
            });
            cx.notify();
            return;
        }

        let target_path = self.target_dir.join(&project_name);
        let template = self.selected_template;
        self.is_creating = true;
        self.status = Some(StatusBanner {
            is_error: false,
            text: format!("Criando projeto '{project_name}'..."),
        });
        cx.notify();

        let task = cx.background_executor().spawn(async move {
            scaffold_project(&target_path, &project_name, template, false).map(|()| target_path)
        });

        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |view, cx| {
                view.is_creating = false;
                match outcome {
                    Ok(created_dir) => {
                        view.screen = LauncherScreen::Home;
                        view.status = Some(StatusBanner {
                            is_error: false,
                            text: format!(
                                "Deck criado em {}. Use `foil dev` na pasta para editar.",
                                created_dir.display()
                            ),
                        });
                    }
                    Err(err) => {
                        view.status = Some(StatusBanner {
                            is_error: true,
                            text: format!("Falha ao criar deck: {err}"),
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .pb_4()
            .border_b_1()
            .border_color(border)
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_xl().font_bold().child("Foil"))
                            .child(
                                div()
                                    .text_xs()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(cx.theme().secondary)
                                    .text_color(muted)
                                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child("AI-First Presentation Engine"),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("nav-home")
                            .small()
                            .label("Início")
                            .when(self.screen == LauncherScreen::Home, |btn| btn.primary())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.screen = LauncherScreen::Home;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("nav-new")
                            .small()
                            .icon(IconName::Plus)
                            .label("Novo")
                            .when(self.screen == LauncherScreen::NewDeck, |btn| btn.primary())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.screen = LauncherScreen::NewDeck;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("nav-help")
                            .small()
                            .icon(IconName::BookOpen)
                            .label("Ajuda")
                            .when(self.screen == LauncherScreen::Help, |btn| btn.primary())
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.screen = LauncherScreen::Help;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("nav-quit")
                            .small()
                            .outline()
                            .icon(IconName::Close)
                            .label("Sair")
                            .on_click(|_, _, cx| cx.quit()),
                    ),
            )
    }

    fn render_status(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let banner = self.status.as_ref()?;
        let bg = if banner.is_error {
            cx.theme().danger
        } else {
            cx.theme().secondary
        };
        let fg = if banner.is_error {
            cx.theme().danger_foreground
        } else {
            cx.theme().foreground
        };
        Some(
            div()
                .w_full()
                .px_3()
                .py_2()
                .rounded_md()
                .bg(bg)
                .text_color(fg)
                .text_sm()
                .child(banner.text.clone()),
        )
    }

    fn render_home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        let card_bg = cx.theme().secondary;

        v_flex()
            .gap_5()
            .w_full()
            .child(
                v_flex()
                    .gap_3()
                    .p_5()
                    .rounded_lg()
                    .border_1()
                    .border_color(border)
                    .bg(card_bg)
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("Comece ou apresente um deck"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child("Crie uma nova apresentação a partir dos templates oficiais ou abra um pacote .foil em modo offline seguro."),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .pt_2()
                            .child(
                                Button::new("home-new")
                                    .primary()
                                    .icon(IconName::Plus)
                                    .label("Novo Deck")
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.screen = LauncherScreen::NewDeck;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("home-open")
                                    .outline()
                                    .icon(IconName::FolderOpen)
                                    .label("Abrir .foil")
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.pick_foil_file(cx);
                                    })),
                            )
                            .child(
                                Button::new("home-help")
                                    .ghost()
                                    .icon(IconName::BookOpen)
                                    .label("Ajuda")
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.screen = LauncherScreen::Help;
                                        cx.notify();
                                    })),
                            )
                            .when(!self.cli_installed, |row| {
                                row.child(
                                    Button::new("home-install-cli")
                                        .outline()
                                        .label("Instalar CLI")
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            view.install_cli(cx);
                                        })),
                                )
                            })
                            .child(
                                Button::new("home-quit")
                                    .ghost()
                                    .icon(IconName::Close)
                                    .label("Sair")
                                    .on_click(|_, _, cx| cx.quit()),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .w_full()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .child("Recentes nesta sessão"),
                    )
                    .when(self.recent_files.is_empty(), |col| {
                        col.child(
                            div()
                                .p_4()
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .text_sm()
                                .text_color(muted)
                                .child("Nenhum arquivo aberto ainda. Você também pode clicar duas vezes em qualquer arquivo .foil no Finder."),
                        )
                    })
                    .when(!self.recent_files.is_empty(), |col| {
                        col.children(self.recent_files.iter().enumerate().map(|(idx, path)| {
                            let path_clone = path.clone();
                            let label = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("deck.foil")
                                .to_string();
                            let full_path = path.display().to_string();
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(Icon::new(IconName::File).small())
                                        .child(
                                            v_flex()
                                                .child(div().text_sm().font_medium().child(label))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(muted)
                                                        .child(full_path),
                                                ),
                                        ),
                                )
                                .child(
                                    Button::new(("open-recent", idx))
                                        .small()
                                        .primary()
                                        .label("Apresentar")
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.open_foil_file(path_clone.clone(), cx);
                                        })),
                                )
                        }))
                    }),
            )
    }

    fn render_new_deck(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        let templates = [
            (TemplateKind::Minimal, "Minimal"),
            (TemplateKind::ThreeD, "3D (Three.js)"),
            (TemplateKind::Pitch, "Pitch"),
        ];

        v_flex()
            .gap_4()
            .w_full()
            .p_5()
            .rounded_lg()
            .border_1()
            .border_color(border)
            .child(div().text_base().font_semibold().child("Criar Novo Deck"))
            .child(
                v_flex()
                    .gap_2()
                    .child(div().text_sm().font_medium().child("Nome do projeto"))
                    .child(Input::new(&self.name_input).cleanable(true)),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(div().text_sm().font_medium().child("Template"))
                    .child(h_flex().gap_2().children(templates.into_iter().map(
                        |(kind, label)| {
                            let selected = self.selected_template == kind;
                            Button::new(("tpl", kind as usize))
                                .small()
                                .label(label)
                                .when(selected, |btn| btn.primary())
                                .when(!selected, |btn| btn.outline())
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.selected_template = kind;
                                    cx.notify();
                                }))
                        },
                    ))),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(div().text_sm().font_medium().child("Pasta de destino"))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(border)
                                    .text_sm()
                                    .text_color(muted)
                                    .child(self.target_dir.display().to_string()),
                            )
                            .child(
                                Button::new("pick-dir")
                                    .outline()
                                    .icon(IconName::Folder)
                                    .label("Escolher Pasta")
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.pick_target_dir(cx);
                                    })),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .pt_2()
                    .child(
                        Button::new("create-submit")
                            .primary()
                            .disabled(self.is_creating)
                            .icon(IconName::Check)
                            .label(if self.is_creating {
                                "Criando..."
                            } else {
                                "Criar Projeto"
                            })
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.create_deck(cx);
                            })),
                    )
                    .child(
                        Button::new("create-cancel")
                            .outline()
                            .label("Voltar")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.screen = LauncherScreen::Home;
                                cx.notify();
                            })),
                    ),
            )
    }

    fn render_help(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        let shortcuts = [
            ("Seta Direita / Espaço", "Avançar slide ou step"),
            ("Seta Esquerda", "Voltar slide ou step"),
            ("F11", "Alternar modo tela cheia"),
            ("Esc", "Sair do modo tela cheia"),
        ];
        let commands = [
            ("foil create <nome>", "Cria um novo projeto de slides"),
            ("foil dev", "Abre preview com hot reload no player"),
            ("foil build", "Gera pacote .foil assinado com SHA-256"),
            ("foil <arquivo.foil>", "Abre e inicia a apresentação direto"),
        ];

        v_flex()
            .gap_4()
            .w_full()
            .child(
                v_flex()
                    .gap_2()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("Atalhos durante a apresentação"),
                    )
                    .children(shortcuts.into_iter().map(|(key, desc)| {
                        h_flex()
                            .justify_between()
                            .text_sm()
                            .child(div().font_medium().child(key))
                            .child(div().text_color(muted).child(desc))
                    })),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(border)
                    .child(div().text_base().font_semibold().child("Comandos CLI"))
                    .children(commands.into_iter().map(|(cmd, desc)| {
                        h_flex()
                            .justify_between()
                            .text_sm()
                            .child(div().font_medium().child(cmd))
                            .child(div().text_color(muted).child(desc))
                    })),
            )
            .child(
                v_flex()
                    .gap_1()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("Segurança & Sandbox"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child("Pacotes .foil rodam 100% offline (connect-src 'none'), com verificação SHA-256 em integrity.json, sanitização de SVG e IPC sem privilégios."),
                    ),
            )
            .child(
                h_flex().child(
                    Button::new("help-back")
                        .primary()
                        .label("Voltar ao Início")
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.screen = LauncherScreen::Home;
                            cx.notify();
                        })),
                ),
            )
    }
}

impl Render for FoilLauncherView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_6()
            .gap_5()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_header(cx))
            .children(self.render_status(cx))
            .child(match self.screen {
                LauncherScreen::Home => self.render_home(cx).into_any_element(),
                LauncherScreen::NewDeck => self.render_new_deck(cx).into_any_element(),
                LauncherScreen::Help => self.render_help(cx).into_any_element(),
            })
    }
}

pub fn file_url_to_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("file://") {
        let path_part = rest.strip_prefix("localhost").unwrap_or(rest);
        if !path_part.starts_with('/') {
            return None;
        }
        let decoded = percent_decode(path_part)?;
        return Some(PathBuf::from(decoded));
    }
    if trimmed.contains("://") {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            let val = u8::from_str_radix(hex, 16).ok()?;
            out.push(val);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn install_cli_to_dir(exe_path: &Path, bin_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(bin_dir)
        .map_err(|err| format!("Falha ao preparar {}: {err}", bin_dir.display()))?;
    let dest = bin_dir.join("foil");
    if dest.exists() || dest.symlink_metadata().is_ok() {
        std::fs::remove_file(&dest)
            .map_err(|err| format!("Falha ao atualizar {}: {err}", dest.display()))?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(exe_path, &dest)
            .map_err(|err| format!("Falha ao criar link em {}: {err}", dest.display()))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(exe_path, &dest)
            .map_err(|err| format!("Falha ao copiar CLI para {}: {err}", dest.display()))?;
    }
    Ok(dest)
}

pub fn install_cli_symlink() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("Falha ao localizar binário do Foil: {err}"))?;
    let system_bin = Path::new("/usr/local/bin");
    if system_bin.is_dir() && let Ok(dest) = install_cli_to_dir(&exe, system_bin) {
        return Ok(dest);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Variável HOME não definida para instalar CLI.".to_string())?;
    let local_bin = home.join(".local").join("bin");
    install_cli_to_dir(&exe, &local_bin)
}

fn is_cli_installed() -> bool {
    let candidates = [
        PathBuf::from("/usr/local/bin/foil"),
        PathBuf::from("/opt/homebrew/bin/foil"),
    ];
    if candidates.iter().any(|p| p.exists()) {
        return true;
    }
    if let Some(home) = std::env::var_os("HOME") {
        let local = PathBuf::from(home).join(".local").join("bin").join("foil");
        if local.exists() {
            return true;
        }
    }
    false
}

pub fn launch_foil_presentation(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Arquivo não encontrado: {}", path.display()));
    }
    let exe = std::env::current_exe()
        .map_err(|err| format!("Falha ao localizar binário do Foil: {err}"))?;
    Command::new(exe)
        .arg("open")
        .arg(path)
        .spawn()
        .map_err(|err| format!("Falha ao iniciar apresentação: {err}"))?;
    Ok(())
}

pub fn run_gui() -> Result<(), CliError> {
    let app = application().with_assets(assets::Assets);
    app.on_open_urls(|urls| {
        for raw in urls {
            if let Some(path) = file_url_to_path(&raw)
                && let Err(err) = launch_foil_presentation(&path)
            {
                eprintln!("foil open error: {err}");
            }
        }
    });

    app.run(|cx| {
        init(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(860.), px(620.)), cx)),
            window_min_size: Some(size(px(680.), px(500.))),
            titlebar: Some(TitlebarOptions {
                title: Some("Foil".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        if let Err(err) = open_window(options, cx, |window, cx| {
            cx.new(|cx| FoilLauncherView::new(window, cx))
        }) {
            eprintln!("foil gui error: {err}");
        }
    });

    Ok(())
}

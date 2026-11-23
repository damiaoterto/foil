use crate::commands::TemplateKind;
use crate::scaffolding::{scaffold_project, CliError};
use foil_packager::FoilArchiveReader;
use gpui_kit::base::StyledExt;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable, Theme, ThemeMode,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

const SKILL_MD: &str = include_str!("../../../assets/skills/foil-engine/SKILL.md");
const SKILL_REF_DESIGN: &str =
    include_str!("../../../assets/skills/foil-engine/references/design-and-typography.md");
const SKILL_REF_MOTION: &str =
    include_str!("../../../assets/skills/foil-engine/references/motion-and-3d.md");
const SKILL_REF_CHARTS: &str =
    include_str!("../../../assets/skills/foil-engine/references/charts-and-diagrams.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLanguage {
    Pt,
    En,
    Es,
    Zh,
    Ru,
}

impl AppLanguage {
    fn code(self) -> &'static str {
        match self {
            Self::Pt => "PT",
            Self::En => "EN",
            Self::Es => "ES",
            Self::Zh => "ZH",
            Self::Ru => "RU",
        }
    }

    fn all() -> [Self; 5] {
        [Self::Pt, Self::En, Self::Es, Self::Zh, Self::Ru]
    }
}

struct UiStrings {
    recent_title: &'static str,
    recent_empty: &'static str,
    recent_hint: &'static str,
    open_foil: &'static str,
    new_project: &'static str,
    home_headline: &'static str,
    home_subtitle: &'static str,
    home_stat_canvas: &'static str,
    home_stat_security: &'static str,
    home_banner_title: &'static str,
    home_banner_desc: &'static str,
    home_banner_btn: &'static str,
    new_title: &'static str,
    new_subtitle: &'static str,
    field_name: &'static str,
    field_template: &'static str,
    field_folder: &'static str,
    choose_folder: &'static str,
    create_btn: &'static str,
    creating_btn: &'static str,
    cancel_btn: &'static str,
    help_title: &'static str,
    help_subtitle: &'static str,
    help_shortcuts: &'static str,
    help_commands: &'static str,
    help_security: &'static str,
    shortcut_next: &'static str,
    shortcut_prev: &'static str,
    shortcut_fs: &'static str,
    shortcut_esc: &'static str,
    cmd_create: &'static str,
    cmd_dev: &'static str,
    cmd_build: &'static str,
    cmd_open: &'static str,
    open_action: &'static str,
    theme_light: &'static str,
    theme_dark: &'static str,
    skill_title: &'static str,
    skill_desc: &'static str,
    skill_install_global: &'static str,
    skill_install_project: &'static str,
}

impl UiStrings {
    fn for_lang(lang: AppLanguage) -> Self {
        match lang {
            AppLanguage::Pt => Self {
                recent_title: "RECENTES",
                recent_empty: "Nenhum deck aberto",
                recent_hint: "Abra um .foil ou dê duplo clique no Finder.",
                open_foil: "Abrir .foil",
                new_project: "Novo Projeto",
                home_headline: "APRESENTAÇÕES EM 120FPS",
                home_subtitle: "Crie slides em Vanilla JS 1920x1080 ou execute pacotes .foil em sandbox 100% offline.",
                home_stat_canvas: "Canvas 16:9",
                home_stat_security: "SHA-256 Offline",
                home_banner_title: "Workflow AI-First com MCP & Hot Reload",
                home_banner_desc: "Use `foil dev` para editar com recarregamento instantâneo e `foil build` para empacotar.",
                home_banner_btn: "Ver Referência",
                new_title: "CRIAR NOVO DECK",
                new_subtitle: "Estrutura Vite + Vanilla JS pronta para 1920x1080.",
                field_name: "NOME DO PROJETO",
                field_template: "TEMPLATE",
                field_folder: "PASTA DE DESTINO",
                choose_folder: "Escolher",
                create_btn: "Criar Projeto",
                creating_btn: "Criando...",
                cancel_btn: "Cancelar",
                help_title: "REFERÊNCIA RÁPIDA",
                help_subtitle: "Atalhos do player e comandos da CLI.",
                help_shortcuts: "ATALHOS DO PLAYER",
                help_commands: "COMANDOS CLI",
                help_security: "Pacotes .foil rodam offline (connect-src 'none') com verificação SHA-256.",
                shortcut_next: "Avançar slide ou step",
                shortcut_prev: "Voltar slide ou step",
                shortcut_fs: "Alternar tela cheia",
                shortcut_esc: "Sair da tela cheia",
                cmd_create: "Cria novo projeto de slides",
                cmd_dev: "Preview com hot reload",
                cmd_build: "Gera pacote .foil assinado",
                cmd_open: "Inicia apresentação direto",
                open_action: "Abrir",
                theme_light: "Claro",
                theme_dark: "Escuro",
                skill_title: "SKILL PARA LLMS (foil-engine)",
                skill_desc: "Instale as diretrizes de design 1920x1080, 2D/3D e MCP no projeto ou globalmente.",
                skill_install_global: "Instalar Global (~/.agents)",
                skill_install_project: "Instalar no Projeto...",
            },
            AppLanguage::En => Self {
                recent_title: "RECENT",
                recent_empty: "No decks opened yet",
                recent_hint: "Open a .foil file or double-click one in Finder.",
                open_foil: "Open .foil",
                new_project: "New Project",
                home_headline: "PRESENTATIONS AT 120FPS",
                home_subtitle: "Build 1920x1080 Vanilla JS slides or run .foil packages in a 100% offline sandbox.",
                home_stat_canvas: "16:9 Canvas",
                home_stat_security: "SHA-256 Offline",
                home_banner_title: "AI-First Workflow with MCP & Hot Reload",
                home_banner_desc: "Run `foil dev` for instant live reload and `foil build` to sign and bundle.",
                home_banner_btn: "Open Reference",
                new_title: "CREATE NEW DECK",
                new_subtitle: "Vite + Vanilla JS scaffold ready for 1920x1080.",
                field_name: "PROJECT NAME",
                field_template: "TEMPLATE",
                field_folder: "TARGET FOLDER",
                choose_folder: "Browse",
                create_btn: "Create Project",
                creating_btn: "Creating...",
                cancel_btn: "Cancel",
                help_title: "QUICK REFERENCE",
                help_subtitle: "Player shortcuts and CLI commands.",
                help_shortcuts: "PLAYER SHORTCUTS",
                help_commands: "CLI COMMANDS",
                help_security: ".foil packages run offline (connect-src 'none') with SHA-256 verification.",
                shortcut_next: "Next slide or step",
                shortcut_prev: "Previous slide or step",
                shortcut_fs: "Toggle fullscreen",
                shortcut_esc: "Exit fullscreen",
                cmd_create: "Scaffold a new slide project",
                cmd_dev: "Live preview with hot reload",
                cmd_build: "Build signed .foil archive",
                cmd_open: "Launch presentation directly",
                open_action: "Open",
                theme_light: "Light",
                theme_dark: "Dark",
                skill_title: "LLM SKILL (foil-engine)",
                skill_desc: "Install 1920x1080 design, 2D/3D blueprints, and MCP guidelines globally or per project.",
                skill_install_global: "Install Global (~/.agents)",
                skill_install_project: "Install in Project...",
            },
            AppLanguage::Es => Self {
                recent_title: "RECIENTES",
                recent_empty: "Ningún deck abierto",
                recent_hint: "Abre un archivo .foil o haz doble clic en Finder.",
                open_foil: "Abrir .foil",
                new_project: "Nuevo Proyecto",
                home_headline: "PRESENTACIONES A 120FPS",
                home_subtitle: "Crea diapositivas Vanilla JS 1920x1080 o ejecuta paquetes .foil en sandbox 100% offline.",
                home_stat_canvas: "Lienzo 16:9",
                home_stat_security: "SHA-256 Offline",
                home_banner_title: "Flujo AI-First con MCP y Hot Reload",
                home_banner_desc: "Usa `foil dev` para recarga instantánea y `foil build` para empaquetar.",
                home_banner_btn: "Ver Referencia",
                new_title: "CREAR NUEVO DECK",
                new_subtitle: "Estructura Vite + Vanilla JS lista para 1920x1080.",
                field_name: "NOMBRE DEL PROYECTO",
                field_template: "PLANTILLA",
                field_folder: "CARPETA DE DESTINO",
                choose_folder: "Elegir",
                create_btn: "Crear Proyecto",
                creating_btn: "Creando...",
                cancel_btn: "Cancelar",
                help_title: "REFERENCIA RÁPIDA",
                help_subtitle: "Atajos del reproductor y comandos CLI.",
                help_shortcuts: "ATAJOS DEL REPRODUCTOR",
                help_commands: "COMANDOS CLI",
                help_security: "Los paquetes .foil se ejecutan offline (connect-src 'none') con verificación SHA-256.",
                shortcut_next: "Siguiente diapositiva o paso",
                shortcut_prev: "Diapositiva o paso anterior",
                shortcut_fs: "Alternar pantalla completa",
                shortcut_esc: "Salir de pantalla completa",
                cmd_create: "Crea un nuevo proyecto",
                cmd_dev: "Vista previa con hot reload",
                cmd_build: "Genera paquete .foil firmado",
                cmd_open: "Inicia la presentación directo",
                open_action: "Abrir",
                theme_light: "Claro",
                theme_dark: "Oscuro",
                skill_title: "SKILL PARA LLMS (foil-engine)",
                skill_desc: "Instala las guías de diseño 1920x1080, 2D/3D y MCP en tu proyecto o de forma global.",
                skill_install_global: "Instalar Global (~/.agents)",
                skill_install_project: "Instalar en Proyecto...",
            },
            AppLanguage::Zh => Self {
                recent_title: "最近打开",
                recent_empty: "暂无打开的演示文稿",
                recent_hint: "打开 .foil 文件或在访达中双击启动。",
                open_foil: "打开 .foil",
                new_project: "新建项目",
                home_headline: "120FPS 原生演示引擎",
                home_subtitle: "使用 1920x1080 Vanilla JS 构建幻灯片，或在 100% 离线沙箱中安全运行 .foil 包。",
                home_stat_canvas: "16:9 画布",
                home_stat_security: "SHA-256 离线",
                home_banner_title: "AI 优先工作流与 MCP 热重载",
                home_banner_desc: "运行 `foil dev` 实时预览，使用 `foil build` 签名打包。",
                home_banner_btn: "查看参考",
                new_title: "创建新演示文稿",
                new_subtitle: "预设 1920x1080 的 Vite + Vanilla JS 项目结构。",
                field_name: "项目名称",
                field_template: "模板",
                field_folder: "目标文件夹",
                choose_folder: "选择",
                create_btn: "创建项目",
                creating_btn: "创建中...",
                cancel_btn: "取消",
                help_title: "快速参考",
                help_subtitle: "播放器快捷键与 CLI 命令。",
                help_shortcuts: "播放器快捷键",
                help_commands: "CLI 命令",
                help_security: ".foil 包在纯离线环境 (connect-src 'none') 中运行，并校验 SHA-256。",
                shortcut_next: "下一页或下一步",
                shortcut_prev: "上一页或上一步",
                shortcut_fs: "切换全屏模式",
                shortcut_esc: "退出全屏模式",
                cmd_create: "创建新幻灯片项目",
                cmd_dev: "开启热重载预览",
                cmd_build: "构建签名 .foil 包",
                cmd_open: "直接启动演示",
                open_action: "打开",
                theme_light: "浅色",
                theme_dark: "深色",
                skill_title: "LLM 技能 (foil-engine)",
                skill_desc: "将 1920x1080 设计规范、2D/3D 蓝图与 MCP 指南安装到项目或全局目录。",
                skill_install_global: "全局安装 (~/.agents)",
                skill_install_project: "安装到项目...",
            },
            AppLanguage::Ru => Self {
                recent_title: "НЕДАВНИЕ",
                recent_empty: "Нет открытых презентаций",
                recent_hint: "Откройте файл .foil или дважды кликните в Finder.",
                open_foil: "Открыть .foil",
                new_project: "Новый проект",
                home_headline: "ПРЕЗЕНТАЦИИ НА 120FPS",
                home_subtitle: "Создавайте слайды 1920x1080 на Vanilla JS или запускайте .foil в оффлайн-песочнице.",
                home_stat_canvas: "Холст 16:9",
                home_stat_security: "SHA-256 Оффлайн",
                home_banner_title: "AI-First рабочий процесс с MCP и Hot Reload",
                home_banner_desc: "Используйте `foil dev` для мгновенного предпросмотра и `foil build` для сборки.",
                home_banner_btn: "Справка",
                new_title: "СОЗДАТЬ ПРЕЗЕНТАЦИЮ",
                new_subtitle: "Шаблон Vite + Vanilla JS для экрана 1920x1080.",
                field_name: "НАЗВАНИЕ ПРОЕКТА",
                field_template: "ШАБЛОН",
                field_folder: "ЦЕЛЕВАЯ ПАПКА",
                choose_folder: "Выбрать",
                create_btn: "Создать проект",
                creating_btn: "Создание...",
                cancel_btn: "Отмена",
                help_title: "КРАТКАЯ СПРАВКА",
                help_subtitle: "Горячие клавиши плеера и команды CLI.",
                help_shortcuts: "ГОРЯЧИЕ КЛАВИШИ",
                help_commands: "КОМАНДЫ CLI",
                help_security: "Пакеты .foil работают оффлайн (connect-src 'none') с проверкой SHA-256.",
                shortcut_next: "Следующий слайд или шаг",
                shortcut_prev: "Предыдущий слайд или шаг",
                shortcut_fs: "Полноэкранный режим",
                shortcut_esc: "Выйти из полноэкранного режима",
                cmd_create: "Создать новый проект слайдов",
                cmd_dev: "Предпросмотр с hot reload",
                cmd_build: "Собрать подписанный .foil",
                cmd_open: "Запустить презентацию сразу",
                open_action: "Открыть",
                theme_light: "Светлая",
                theme_dark: "Тёмная",
                skill_title: "НАВЫК ДЛЯ LLM (foil-engine)",
                skill_desc: "Установите гайды по дизайну 1920x1080, 2D/3D и MCP в проект или глобально.",
                skill_install_global: "Установить глобально",
                skill_install_project: "Установить в проект...",
            },
        }
    }
}

struct Palette {
    rail: Rgba,
    rail_active_bg: Rgba,
    rail_active_fg: Rgba,
    rail_hover: Rgba,
    rail_ink: Rgba,
    canvas: Rgba,
    surface_rail: Rgba,
    surface_card: Rgba,
    surface_elevated: Rgba,
    surface_black: Rgba,
    ink: Rgba,
    muted: Rgba,
    border: Rgba,
    accent_bg: Rgba,
    accent_fg: Rgba,
}

impl Palette {
    fn for_mode(is_dark: bool) -> Self {
        if is_dark {
            Self {
                rail: rgb(0x1A1A1A),
                rail_active_bg: rgb(0xF5F5F5),
                rail_active_fg: rgb(0x1A1A1A),
                rail_hover: rgb(0x2E2E2E),
                rail_ink: rgb(0xA3A3A3),
                canvas: rgb(0x262626),
                surface_rail: rgb(0x1F1F1F),
                surface_card: rgb(0x2E2E2E),
                surface_elevated: rgb(0x383838),
                surface_black: rgb(0x171717),
                ink: rgb(0xF5F5F5),
                muted: rgb(0xA3A3A3),
                border: rgb(0x3F3F3F),
                accent_bg: rgb(0xF5F5F5),
                accent_fg: rgb(0x171717),
            }
        } else {
            Self {
                rail: rgb(0xE5E5E5),
                rail_active_bg: rgb(0x262626),
                rail_active_fg: rgb(0xFFFFFF),
                rail_hover: rgb(0xD4D4D4),
                rail_ink: rgb(0x525252),
                canvas: rgb(0xF5F5F5),
                surface_rail: rgb(0xEBEBEB),
                surface_card: rgb(0xFFFFFF),
                surface_elevated: rgb(0xE8E8E8),
                surface_black: rgb(0x262626),
                ink: rgb(0x171717),
                muted: rgb(0x525252),
                border: rgb(0xD4D4D4),
                accent_bg: rgb(0x262626),
                accent_fg: rgb(0xFFFFFF),
            }
        }
    }
}

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
    lang: AppLanguage,
    logo_image: Arc<Image>,
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
        let logo_image = Arc::new(Image::from_bytes(
            ImageFormat::Png,
            include_bytes!("../../../assets/icons/foil-icon-128.png").to_vec(),
        ));
        Self {
            screen: LauncherScreen::Home,
            lang: AppLanguage::Pt,
            logo_image,
            name_input,
            selected_template: TemplateKind::Minimal,
            target_dir,
            is_creating: false,
            cli_installed: is_cli_installed(),
            recent_files: Vec::new(),
            status: None,
        }
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next_mode = if cx.theme().mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        Theme::change(next_mode, None, cx);
        cx.notify();
    }

    fn set_language(&mut self, lang: AppLanguage, cx: &mut Context<Self>) {
        self.lang = lang;
        cx.notify();
    }

    fn install_skill_global_action(&mut self, cx: &mut Context<Self>) {
        match install_skill_global() {
            Ok(dest) => {
                self.status = Some(StatusBanner {
                    is_error: false,
                    text: format!("Skill instalada globalmente em {}", dest.display()),
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

    fn pick_project_for_skill(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Selecionar pasta do projeto para instalar a skill".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(project_dir) = paths.into_iter().next() else {
                return;
            };
            let outcome = install_skill_to_dir(&project_dir.join(".agents").join("skills"));
            let _ = this.update(cx, |view, cx| {
                match outcome {
                    Ok(dest) => {
                        view.status = Some(StatusBanner {
                            is_error: false,
                            text: format!("Skill instalada no projeto em {}", dest.display()),
                        });
                    }
                    Err(err) => {
                        view.status = Some(StatusBanner {
                            is_error: true,
                            text: err,
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
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

    fn render_rail_nav_button(
        &self,
        id: &'static str,
        icon: IconName,
        target: LauncherScreen,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.screen == target;
        div()
            .id(id)
            .size(px(36.))
            .rounded(px(10.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when(active, |el| el.bg(p.rail_active_bg).text_color(p.rail_active_fg))
            .when(!active, |el| {
                el.text_color(p.rail_ink)
                    .hover(|s| s.bg(p.rail_hover).text_color(p.ink))
            })
            .on_click(cx.listener(move |view, _, _, cx| {
                view.screen = target;
                cx.notify();
            }))
            .child(Icon::new(icon))
    }

    fn render_icon_rail(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();

        v_flex()
            .w(px(54.))
            .h_full()
            .py_4()
            .items_center()
            .justify_between()
            .bg(p.rail)
            .border_r_1()
            .border_color(p.border)
            .child(
                v_flex()
                    .items_center()
                    .gap_4()
                    .child(img(self.logo_image.clone()).size(px(36.)))
                    .child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(self.render_rail_nav_button(
                                "nav-home",
                                IconName::FolderOpen,
                                LauncherScreen::Home,
                                p,
                                cx,
                            ))
                            .child(self.render_rail_nav_button(
                                "nav-new",
                                IconName::Plus,
                                LauncherScreen::NewDeck,
                                p,
                                cx,
                            ))
                            .child(self.render_rail_nav_button(
                                "nav-help",
                                IconName::BookOpen,
                                LauncherScreen::Help,
                                p,
                                cx,
                            )),
                    ),
            )
            .child(
                v_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("theme-toggle")
                            .size(px(36.))
                            .rounded(px(10.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(p.rail_ink)
                            .hover(|s| s.bg(p.rail_hover).text_color(p.ink))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.toggle_theme(cx);
                            }))
                            .child(Icon::new(if is_dark {
                                IconName::Sun
                            } else {
                                IconName::Moon
                            })),
                    )
                    .when(!self.cli_installed, |col| {
                        col.child(
                            div()
                                .id("sidebar-install-cli")
                                .size(px(36.))
                                .rounded(px(10.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .text_color(p.ink)
                                .hover(|s| s.bg(p.rail_hover))
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.install_cli(cx);
                                }))
                                .child(Icon::new(IconName::Settings)),
                        )
                    })
                    .child(
                        div()
                            .id("nav-quit")
                            .size(px(36.))
                            .rounded(px(10.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(p.rail_ink)
                            .hover(|s| s.bg(p.rail_hover).text_color(p.ink))
                            .on_click(|_, _, cx| cx.quit())
                            .child(Icon::new(IconName::Close)),
                    ),
            )
    }

    fn render_recents_panel(
        &self,
        p: &Palette,
        s: &UiStrings,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .w(px(216.))
            .h_full()
            .p_4()
            .justify_between()
            .bg(p.surface_rail)
            .border_r_1()
            .border_color(p.border)
            .child(
                v_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(p.ink)
                                    .child(s.recent_title),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(50.))
                                    .bg(p.surface_elevated)
                                    .text_color(p.ink)
                                    .text_xs()
                                    .font_bold()
                                    .child(self.recent_files.len().to_string()),
                            ),
                    )
                    .when(self.recent_files.is_empty(), |col| {
                        col.child(
                            v_flex()
                                .gap_1p5()
                                .p_4()
                                .rounded(px(14.))
                                .bg(p.surface_card)
                                .border_1()
                                .border_color(p.border)
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(p.ink)
                                        .child(s.recent_empty),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(p.muted)
                                        .child(s.recent_hint),
                                ),
                        )
                    })
                    .when(!self.recent_files.is_empty(), |col| {
                        col.child(v_flex().gap_2().children(
                            self.recent_files.iter().enumerate().map(|(idx, path)| {
                                let path_clone = path.clone();
                                let label = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("deck.foil")
                                    .to_string();
                                let full_path = path.display().to_string();
                                v_flex()
                                    .id(("open-recent", idx))
                                    .gap_1p5()
                                    .p_3()
                                    .rounded(px(14.))
                                    .bg(p.surface_card)
                                    .border_1()
                                    .border_color(p.border)
                                    .cursor_pointer()
                                    .hover(|st| st.bg(p.surface_elevated))
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.open_foil_file(path_clone.clone(), cx);
                                    }))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .text_sm()
                                                    .font_semibold()
                                                    .text_color(p.ink)
                                                    .child(label),
                                            )
                                            .child(
                                                div()
                                                    .px_2()
                                                    .py_0p5()
                                                    .rounded(px(8.))
                                                    .bg(p.accent_bg)
                                                    .text_color(p.accent_fg)
                                                    .text_xs()
                                                    .font_semibold()
                                                    .child(s.open_action),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(p.muted)
                                            .child(full_path),
                                    )
                            }),
                        ))
                    }),
            )
            .child(
                h_flex()
                    .id("rail-open-foil")
                    .w_full()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .py_2p5()
                    .rounded(px(12.))
                    .bg(p.surface_elevated)
                    .border_1()
                    .border_color(p.border)
                    .text_color(p.ink)
                    .text_sm()
                    .font_semibold()
                    .cursor_pointer()
                    .hover(|st| st.bg(p.surface_card))
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.pick_foil_file(cx);
                    }))
                    .child(Icon::new(IconName::FolderOpen).small())
                    .child(s.open_foil),
            )
    }

    fn render_top_bar(
        &self,
        p: &Palette,
        s: &UiStrings,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();

        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap_2()
            .when(!compact, |row| {
                row.child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_bold()
                                .text_color(p.muted)
                                .child("FOIL ENGINE"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded(px(50.))
                                .bg(p.surface_card)
                                .border_1()
                                .border_color(p.border)
                                .text_xs()
                                .text_color(p.muted)
                                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                        ),
                )
            })
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .when(compact, |el| el.w_full().justify_between())
                    .child(
                        h_flex()
                            .p_1()
                            .gap_1()
                            .rounded(px(10.))
                            .bg(p.surface_card)
                            .border_1()
                            .border_color(p.border)
                            .children(AppLanguage::all().into_iter().map(|lang| {
                                let selected = self.lang == lang;
                                div()
                                    .id(("lang", lang as usize))
                                    .px_2()
                                    .py_0p5()
                                    .rounded(px(7.))
                                    .text_xs()
                                    .font_bold()
                                    .cursor_pointer()
                                    .when(selected, |el| {
                                        el.bg(p.accent_bg).text_color(p.accent_fg)
                                    })
                                    .when(!selected, |el| {
                                        el.text_color(p.muted)
                                            .hover(|st| st.text_color(p.ink))
                                    })
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.set_language(lang, cx);
                                    }))
                                    .child(lang.code())
                            })),
                    )
                    .child(
                        h_flex()
                            .id("top-theme-toggle")
                            .items_center()
                            .gap_1p5()
                            .px_2p5()
                            .py_1()
                            .rounded(px(10.))
                            .bg(p.surface_card)
                            .border_1()
                            .border_color(p.border)
                            .text_xs()
                            .font_semibold()
                            .text_color(p.ink)
                            .cursor_pointer()
                            .hover(|st| st.bg(p.surface_elevated))
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.toggle_theme(cx);
                            }))
                            .child(
                                Icon::new(if is_dark {
                                    IconName::Sun
                                } else {
                                    IconName::Moon
                                })
                                .small(),
                            )
                            .child(if is_dark { s.theme_light } else { s.theme_dark }),
                    ),
            )
    }

    fn render_status(&self, p: &Palette, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let banner = self.status.as_ref()?;
        let bg: Hsla = if banner.is_error {
            cx.theme().danger
        } else {
            p.surface_card.into()
        };
        let fg: Hsla = if banner.is_error {
            cx.theme().danger_foreground
        } else {
            p.ink.into()
        };
        Some(
            div()
                .w_full()
                .px_4()
                .py_2p5()
                .rounded(px(12.))
                .border_1()
                .border_color(p.border)
                .bg(bg)
                .text_color(fg)
                .text_sm()
                .child(banner.text.clone()),
        )
    }

    fn render_home(
        &self,
        p: &Palette,
        s: &UiStrings,
        compact: bool,
        show_recents_rail: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cta_row = h_flex()
            .gap_3()
            .pt_1()
            .child(
                h_flex()
                    .id("home-new")
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .px_5()
                    .py_2p5()
                    .rounded(px(12.))
                    .bg(p.accent_bg)
                    .text_color(p.accent_fg)
                    .text_sm()
                    .font_bold()
                    .cursor_pointer()
                    .when(compact, |el| el.flex_1())
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.screen = LauncherScreen::NewDeck;
                        cx.notify();
                    }))
                    .child(Icon::new(IconName::Plus).small())
                    .child(s.new_project),
            )
            .child(
                h_flex()
                    .id("home-open")
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .px_5()
                    .py_2p5()
                    .rounded(px(12.))
                    .bg(p.surface_elevated)
                    .border_1()
                    .border_color(p.border)
                    .text_color(p.ink)
                    .text_sm()
                    .font_bold()
                    .cursor_pointer()
                    .hover(|st| st.bg(p.surface_card))
                    .when(compact, |el| el.flex_1())
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.pick_foil_file(cx);
                    }))
                    .child(Icon::new(IconName::FolderOpen).small())
                    .child(s.open_foil),
            );

        let stat_canvas = v_flex()
            .when(!compact, |el| el.flex_1().min_w_0())
            .when(compact, |el| el.w_full())
            .gap_1()
            .p_5()
            .rounded(px(16.))
            .bg(p.surface_card)
            .border_1()
            .border_color(p.border)
            .text_color(p.ink)
            .child(div().text_2xl().font_bold().child("1920×1080"))
            .child(div().text_xs().text_color(p.muted).child(s.home_stat_canvas));

        let stat_security = v_flex()
            .when(!compact, |el| el.flex_1().min_w_0())
            .when(compact, |el| el.w_full())
            .gap_1()
            .p_5()
            .rounded(px(16.))
            .bg(p.surface_card)
            .border_1()
            .border_color(p.border)
            .text_color(p.ink)
            .child(div().text_2xl().font_bold().child("0 PORTAS"))
            .child(div().text_xs().text_color(p.muted).child(s.home_stat_security));

        let stats_block = if compact {
            v_flex()
                .gap_3()
                .w_full()
                .child(stat_canvas)
                .child(stat_security)
                .into_any_element()
        } else {
            h_flex()
                .gap_4()
                .w_full()
                .child(stat_canvas)
                .child(stat_security)
                .into_any_element()
        };

        let banner_text = v_flex()
            .gap_1()
            .when(!compact, |el| el.flex_1().min_w_0())
            .child(
                div()
                    .text_base()
                    .font_bold()
                    .child(s.home_banner_title),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xA3A3A3))
                    .child(s.home_banner_desc),
            );

        let banner_btn = div()
            .id("home-help-banner")
            .flex_shrink_0()
            .px_4()
            .py_2()
            .rounded(px(10.))
            .bg(rgb(0xF5F5F5))
            .text_color(rgb(0x171717))
            .text_xs()
            .font_bold()
            .cursor_pointer()
            .on_click(cx.listener(|view, _, _, cx| {
                view.screen = LauncherScreen::Help;
                cx.notify();
            }))
            .child(s.home_banner_btn);

        let showcase_band = if compact {
            v_flex()
                .w_full()
                .items_start()
                .gap_3()
                .p_5()
                .rounded(px(16.))
                .bg(p.surface_black)
                .border_1()
                .border_color(p.border)
                .text_color(rgb(0xF5F5F5))
                .child(banner_text)
                .child(banner_btn)
                .into_any_element()
        } else {
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .gap_4()
                .p_5()
                .rounded(px(16.))
                .bg(p.surface_black)
                .border_1()
                .border_color(p.border)
                .text_color(rgb(0xF5F5F5))
                .child(banner_text)
                .child(banner_btn)
                .into_any_element()
        };

        v_flex()
            .gap_5()
            .w_full()
            .child(
                v_flex()
                    .gap_4()
                    .p_6()
                    .rounded(px(18.))
                    .bg(p.surface_card)
                    .border_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(p.ink)
                            .child(s.home_headline),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.muted)
                            .child(s.home_subtitle),
                    )
                    .child(cta_row),
            )
            .child(stats_block)
            .child(showcase_band)
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .p_5()
                    .rounded(px(16.))
                    .bg(p.surface_card)
                    .border_1()
                    .border_color(p.border)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(p.ink)
                                    .child(s.skill_title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.muted)
                                    .child(s.skill_desc),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .id("skill-install-global")
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .px_3p5()
                                    .py_2()
                                    .rounded(px(10.))
                                    .bg(p.accent_bg)
                                    .text_color(p.accent_fg)
                                    .text_xs()
                                    .font_bold()
                                    .cursor_pointer()
                                    .when(compact, |el| el.flex_1())
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.install_skill_global_action(cx);
                                    }))
                                    .child(Icon::new(IconName::Check).small())
                                    .child(s.skill_install_global),
                            )
                            .child(
                                h_flex()
                                    .id("skill-install-project")
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .px_3p5()
                                    .py_2()
                                    .rounded(px(10.))
                                    .bg(p.surface_elevated)
                                    .border_1()
                                    .border_color(p.border)
                                    .text_color(p.ink)
                                    .text_xs()
                                    .font_bold()
                                    .cursor_pointer()
                                    .hover(|st| st.bg(p.surface_card))
                                    .when(compact, |el| el.flex_1())
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.pick_project_for_skill(cx);
                                    }))
                                    .child(Icon::new(IconName::Folder).small())
                                    .child(s.skill_install_project),
                            ),
                    ),
            )
            .when(!show_recents_rail, |col| {
                col.child(
                    v_flex()
                        .gap_2()
                        .w_full()
                        .p_4()
                        .rounded(px(16.))
                        .bg(p.surface_card)
                        .border_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .text_xs()
                                .font_bold()
                                .text_color(p.muted)
                                .child(s.recent_title),
                        )
                        .when(self.recent_files.is_empty(), |inner| {
                            inner.child(
                                div()
                                    .text_xs()
                                    .text_color(p.muted)
                                    .child(s.recent_empty),
                            )
                        })
                        .when(!self.recent_files.is_empty(), |inner| {
                            inner.children(self.recent_files.iter().enumerate().map(|(idx, path)| {
                                let path_clone = path.clone();
                                let label = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("deck.foil")
                                    .to_string();
                                h_flex()
                                    .id(("inline-recent", idx))
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .py_1p5()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.open_foil_file(path_clone.clone(), cx);
                                    }))
                                    .child(div().text_sm().text_color(p.ink).child(label))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_bold()
                                            .text_color(p.ink)
                                            .child(s.open_action),
                                    )
                            }))
                        }),
                )
            })
    }

    fn render_new_deck(&self, p: &Palette, s: &UiStrings, cx: &mut Context<Self>) -> impl IntoElement {
        let templates = [
            (TemplateKind::Minimal, "Minimal"),
            (TemplateKind::ThreeD, "3D (Three.js)"),
            (TemplateKind::Pitch, "Pitch"),
        ];

        v_flex()
            .gap_5()
            .w_full()
            .max_w(px(580.))
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(p.ink)
                            .child(s.new_title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.muted)
                            .child(s.new_subtitle),
                    ),
            )
            .child(
                v_flex()
                    .gap_4()
                    .p_6()
                    .rounded(px(16.))
                    .bg(p.surface_card)
                    .border_1()
                    .border_color(p.border)
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(p.muted)
                                    .child(s.field_name),
                            )
                            .child(Input::new(&self.name_input).cleanable(true)),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(p.muted)
                                    .child(s.field_template),
                            )
                            .child(h_flex().gap_2().children(templates.into_iter().map(
                                |(kind, label)| {
                                    let selected = self.selected_template == kind;
                                    div()
                                        .id(("tpl", kind as usize))
                                        .px_3p5()
                                        .py_2()
                                        .rounded(px(10.))
                                        .border_1()
                                        .text_sm()
                                        .font_semibold()
                                        .cursor_pointer()
                                        .when(selected, |el| {
                                            el.bg(p.accent_bg)
                                                .border_color(p.accent_bg)
                                                .text_color(p.accent_fg)
                                        })
                                        .when(!selected, |el| {
                                            el.bg(p.surface_elevated)
                                                .border_color(p.border)
                                                .text_color(p.ink)
                                                .hover(|st| st.bg(p.surface_card))
                                        })
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            view.selected_template = kind;
                                            cx.notify();
                                        }))
                                        .child(label)
                                },
                            ))),
                    )
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold()
                                    .text_color(p.muted)
                                    .child(s.field_folder),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .px_3()
                                            .py_2()
                                            .rounded(px(10.))
                                            .border_1()
                                            .border_color(p.border)
                                            .bg(p.canvas)
                                            .text_sm()
                                            .text_color(p.muted)
                                            .child(self.target_dir.display().to_string()),
                                    )
                                    .child(
                                        h_flex()
                                            .id("pick-dir")
                                            .flex_shrink_0()
                                            .items_center()
                                            .gap_1p5()
                                            .px_3p5()
                                            .py_2()
                                            .rounded(px(10.))
                                            .bg(p.surface_elevated)
                                            .border_1()
                                            .border_color(p.border)
                                            .text_sm()
                                            .font_semibold()
                                            .text_color(p.ink)
                                            .cursor_pointer()
                                            .hover(|st| st.bg(p.surface_card))
                                            .on_click(cx.listener(|view, _, _, cx| {
                                                view.pick_target_dir(cx);
                                            }))
                                            .child(Icon::new(IconName::Folder).small())
                                            .child(s.choose_folder),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .pt_2()
                            .child(
                                h_flex()
                                    .id("create-submit")
                                    .items_center()
                                    .gap_2()
                                    .px_5()
                                    .py_2p5()
                                    .rounded(px(12.))
                                    .bg(p.accent_bg)
                                    .text_color(p.accent_fg)
                                    .text_sm()
                                    .font_bold()
                                    .cursor_pointer()
                                    .when(!self.is_creating, |el| {
                                        el.on_click(cx.listener(|view, _, _, cx| {
                                            view.create_deck(cx);
                                        }))
                                    })
                                    .child(Icon::new(IconName::Check).small())
                                    .child(if self.is_creating {
                                        s.creating_btn
                                    } else {
                                        s.create_btn
                                    }),
                            )
                            .child(
                                div()
                                    .id("create-cancel")
                                    .px_4()
                                    .py_2p5()
                                    .rounded(px(12.))
                                    .bg(p.surface_elevated)
                                    .border_1()
                                    .border_color(p.border)
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .hover(|st| st.bg(p.surface_card))
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.screen = LauncherScreen::Home;
                                        cx.notify();
                                    }))
                                    .child(s.cancel_btn),
                            ),
                    ),
            )
    }

    fn render_help(
        &self,
        p: &Palette,
        s: &UiStrings,
        compact: bool,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let shortcuts = [
            ("→ / Space", s.shortcut_next),
            ("←", s.shortcut_prev),
            ("F11", s.shortcut_fs),
            ("Esc", s.shortcut_esc),
        ];
        let commands = [
            ("foil create <name>", s.cmd_create),
            ("foil dev", s.cmd_dev),
            ("foil build", s.cmd_build),
            ("foil <file.foil>", s.cmd_open),
        ];

        let shortcuts_card = v_flex()
            .when(!compact, |el| el.flex_1().min_w_0())
            .when(compact, |el| el.w_full())
            .gap_2p5()
            .p_5()
            .rounded(px(16.))
            .bg(p.surface_card)
            .border_1()
            .border_color(p.border)
            .child(
                div()
                    .text_xs()
                    .font_bold()
                    .text_color(p.muted)
                    .child(s.help_shortcuts),
            )
            .children(shortcuts.into_iter().map(|(key, desc)| {
                h_flex()
                    .py_1p5()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(p.border)
                    .child(div().min_w_0().text_xs().text_color(p.ink).child(desc))
                    .child(
                        div()
                            .flex_shrink_0()
                            .px_2()
                            .py_0p5()
                            .rounded(px(6.))
                            .bg(p.surface_elevated)
                            .border_1()
                            .border_color(p.border)
                            .text_color(p.ink)
                            .text_xs()
                            .font_bold()
                            .child(key),
                    )
            }));

        let commands_card = v_flex()
            .when(!compact, |el| el.flex_1().min_w_0())
            .when(compact, |el| el.w_full())
            .gap_2p5()
            .p_5()
            .rounded(px(16.))
            .bg(p.surface_card)
            .border_1()
            .border_color(p.border)
            .child(
                div()
                    .text_xs()
                    .font_bold()
                    .text_color(p.muted)
                    .child(s.help_commands),
            )
            .children(commands.into_iter().map(|(cmd, desc)| {
                v_flex()
                    .py_1p5()
                    .gap_0p5()
                    .border_b_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .text_xs()
                            .font_bold()
                            .text_color(p.ink)
                            .child(cmd),
                    )
                    .child(div().text_xs().text_color(p.muted).child(desc))
            }));

        let cards_grid = if compact {
            v_flex()
                .gap_4()
                .w_full()
                .child(shortcuts_card)
                .child(commands_card)
                .into_any_element()
        } else {
            h_flex()
                .gap_4()
                .w_full()
                .items_start()
                .child(shortcuts_card)
                .child(commands_card)
                .into_any_element()
        };

        v_flex()
            .gap_5()
            .w_full()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(p.ink)
                            .child(s.help_title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.muted)
                            .child(s.help_subtitle),
                    ),
            )
            .child(cards_grid)
            .child(
                div()
                    .p_4()
                    .rounded(px(16.))
                    .bg(p.surface_black)
                    .border_1()
                    .border_color(p.border)
                    .text_color(rgb(0xF5F5F5))
                    .text_xs()
                    .child(s.help_security),
            )
    }
}

impl Render for FoilLauncherView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::for_mode(cx.theme().mode.is_dark());
        let s = UiStrings::for_lang(self.lang);
        let width = window.viewport_size().width;
        let show_recents_rail = width >= px(840.);
        let compact = width < px(640.);

        h_flex()
            .size_full()
            .bg(p.canvas)
            .text_color(p.ink)
            .child(self.render_icon_rail(&p, cx))
            .when(show_recents_rail, |row| {
                row.child(self.render_recents_panel(&p, &s, cx))
            })
            .child(
                v_flex()
                    .id("main-scroll")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .when(compact, |el| el.px_4().py_4().gap_4())
                    .when(!compact, |el| el.px_6().py_5().gap_5())
                    .child(self.render_top_bar(&p, &s, compact, cx))
                    .children(self.render_status(&p, cx))
                    .child(match self.screen {
                        LauncherScreen::Home => {
                            self.render_home(&p, &s, compact, show_recents_rail, cx).into_any_element()
                        }
                        LauncherScreen::NewDeck => {
                            self.render_new_deck(&p, &s, cx).into_any_element()
                        }
                        LauncherScreen::Help => {
                            self.render_help(&p, &s, compact, cx).into_any_element()
                        }
                    }),
            )
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

pub fn install_skill_to_dir(skills_root: &Path) -> Result<PathBuf, String> {
    let dest = skills_root.join("foil-engine");
    let refs_dir = dest.join("references");
    std::fs::create_dir_all(&refs_dir)
        .map_err(|err| format!("Falha ao preparar {}: {err}", refs_dir.display()))?;
    std::fs::write(dest.join("SKILL.md"), SKILL_MD)
        .map_err(|err| format!("Falha ao gravar SKILL.md: {err}"))?;
    std::fs::write(refs_dir.join("design-and-typography.md"), SKILL_REF_DESIGN)
        .map_err(|err| format!("Falha ao gravar design-and-typography.md: {err}"))?;
    std::fs::write(refs_dir.join("motion-and-3d.md"), SKILL_REF_MOTION)
        .map_err(|err| format!("Falha ao gravar motion-and-3d.md: {err}"))?;
    std::fs::write(refs_dir.join("charts-and-diagrams.md"), SKILL_REF_CHARTS)
        .map_err(|err| format!("Falha ao gravar charts-and-diagrams.md: {err}"))?;
    Ok(dest)
}

pub fn install_skill_global() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Variável HOME não definida para instalar skill global.".to_string())?;
    install_skill_to_dir(&home.join(".agents").join("skills"))
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
        Theme::change(ThemeMode::Dark, None, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1040.), px(660.)), cx)),
            window_min_size: Some(size(px(480.), px(420.))),
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

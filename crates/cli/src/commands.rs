use crate::scaffolding::{scaffold_project, CliError};
use clap::{Parser, Subcommand, ValueEnum};
use foil_mcp_server::run_mcp_stdio_server;
use foil_packager::{build_deck, BuildReport, BuildTarget, FoilArchiveReader};
use foil_player::{run_player, PlayerConfig, PlayerMode, DEFAULT_DEV_BRIDGE_PORT};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[derive(Parser, Debug)]
#[command(name = "foil", version, about = "AI-First Presentation Engine")]
pub struct FoilCli {
    #[command(subcommand)]
    pub command: FoilCommand,
}

#[derive(Subcommand, Debug)]
pub enum FoilCommand {
    Create {
        name: String,
        #[arg(long, value_enum, default_value_t = TemplateKind::Minimal)]
        template: TemplateKind,
        #[arg(long, default_value_t = false)]
        no_install: bool,
    },
    Init {
        #[arg(long, value_enum, default_value_t = TemplateKind::Minimal)]
        template: TemplateKind,
        #[arg(long, default_value_t = false)]
        no_install: bool,
    },
    Dev {
        #[arg(long, default_value_t = false)]
        headless: bool,
        #[arg(long, default_value_t = false)]
        browser: bool,
        #[arg(long, default_value_t = 5173)]
        port: u16,
    },
    Build {
        #[arg(long, value_enum, default_value_t = CliBuildTarget::Foil)]
        target: CliBuildTarget,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Open {
        file: PathBuf,
        #[arg(long, default_value_t = false)]
        browser: bool,
    },
    Mcp,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateKind {
    Minimal,
    #[value(name = "3d")]
    ThreeD,
    Pitch,
}

impl TemplateKind {
    pub fn as_theme_name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::ThreeD => "3d",
            Self::Pitch => "pitch",
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliBuildTarget {
    Foil,
    Web,
}

impl From<CliBuildTarget> for BuildTarget {
    fn from(value: CliBuildTarget) -> Self {
        match value {
            CliBuildTarget::Foil => Self::FoilArchive,
            CliBuildTarget::Web => Self::StaticWeb,
        }
    }
}

pub fn execute_cli(cli: FoilCli, cwd: &Path) -> Result<(), CliError> {
    match cli.command {
        FoilCommand::Create {
            name,
            template,
            no_install,
        } => {
            let target_dir = cwd.join(&name);
            scaffold_project(&target_dir, &name, template, no_install)
        }
        FoilCommand::Init {
            template,
            no_install,
        } => {
            let project_name = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("foil-deck");
            scaffold_project(cwd, project_name, template, no_install)
        }
        FoilCommand::Dev {
            headless,
            browser,
            port,
        } => {
            let mut vite_child = spawn_vite_dev_server(cwd, port, browser)?;
            let dev_url = format!("http://127.0.0.1:{port}");
            if browser {
                let _ = vite_child.wait();
                Ok(())
            } else {
                let res = run_player(PlayerConfig {
                    title: "Foil Dev".to_string(),
                    mode: PlayerMode::DevServer {
                        dev_url,
                        bridge_port: DEFAULT_DEV_BRIDGE_PORT,
                    },
                    fullscreen: false,
                    headless,
                });
                let _ = vite_child.kill();
                Ok(res?)
            }
        }
        FoilCommand::Build { target, output } => {
            let default_out = match target {
                CliBuildTarget::Foil => cwd.join("deck.foil"),
                CliBuildTarget::Web => cwd.join("dist-web"),
            };
            let out_path = output.unwrap_or(default_out);
            let report = build_deck(cwd, &out_path, target.into())?;
            println!("{}", format_build_report(&report));
            Ok(())
        }
        FoilCommand::Open { file, browser } => {
            let reader = FoilArchiveReader::from_path(&file)?;
            if browser {
                println!(
                    "Verified {} files in {}. Ready for @foil/web-player.",
                    reader.file_count(),
                    file.display()
                );
                Ok(())
            } else {
                Ok(run_player(PlayerConfig {
                    title: "Foil Player".to_string(),
                    mode: PlayerMode::Archive(reader),
                    fullscreen: false,
                    headless: false,
                })?)
            }
        }
        FoilCommand::Mcp => Ok(run_mcp_stdio_server(cwd.to_path_buf())?),
    }
}

pub fn format_build_report(report: &BuildReport) -> String {
    format!(
        "Foil Build Complete: raw={}B optimized={}B final={}B converted_images={} verified_hashes={} output={}",
        report.raw_size_bytes,
        report.optimized_size_bytes,
        report.archive_size_bytes,
        report.converted_images,
        report.verified_hashes,
        report.output_path.display()
    )
}

fn spawn_vite_dev_server(cwd: &Path, port: u16, open_browser: bool) -> Result<Child, CliError> {
    let mut cmd = Command::new("bunx");
    cmd.arg("vite")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(cwd);
    if open_browser {
        cmd.arg("--open");
    }
    Ok(cmd.spawn()?)
}

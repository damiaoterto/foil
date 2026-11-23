pub mod commands;
pub mod gui;
pub mod scaffolding;

use clap::Parser;
use commands::{execute_cli, FoilCli, FoilCommand};

fn main() {
    let cli = FoilCli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let result = match (cli.command, cli.file) {
        (Some(command), _) => execute_cli(command, &cwd),
        (None, Some(file)) => execute_cli(
            FoilCommand::Open {
                file,
                browser: false,
            },
            &cwd,
        ),
        (None, None) => gui::run_gui(),
    };
    if let Err(err) = result {
        eprintln!("foil error: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::commands::{execute_cli, CliBuildTarget, FoilCli, FoilCommand, TemplateKind};
    use clap::Parser;
    use foil_packager::FoilArchiveReader;
    use tempfile::tempdir;

    #[test]
    fn test_cli_parses_all_subcommands_correctly() {
        let create_cli =
            FoilCli::try_parse_from(["foil", "create", "my-deck", "--template", "3d", "--no-install"])
                .expect("parse create");
        assert!(matches!(
            create_cli.command,
            Some(FoilCommand::Create {
                template: TemplateKind::ThreeD,
                no_install: true,
                ..
            })
        ));

        let build_web =
            FoilCli::try_parse_from(["foil", "build", "--target", "web"]).expect("parse build web");
        assert!(matches!(
            build_web.command,
            Some(FoilCommand::Build {
                target: CliBuildTarget::Web,
                ..
            })
        ));
    }

    #[test]
    fn test_cli_create_and_build_foil_and_web_targets() {
        let dir = tempdir().expect("tempdir");
        let create_cmd = FoilCommand::Create {
            name: "demo-deck".to_string(),
            template: TemplateKind::Pitch,
            no_install: true,
        };
        execute_cli(create_cmd, dir.path()).expect("execute create --no-install");

        let deck_dir = dir.path().join("demo-deck");
        assert!(deck_dir.join("manifest.json").is_file());
        assert!(deck_dir.join("vite.config.ts").is_file());
        assert!(deck_dir.join("src/styles/theme.css").is_file());
        assert!(deck_dir.join("src/slides/01-capa.js").is_file());

        std::fs::create_dir_all(deck_dir.join("dist")).expect("mkdir dist");
        std::fs::copy(deck_dir.join("index.html"), deck_dir.join("dist/index.html"))
            .expect("copy index.html");

        let out_foil = deck_dir.join("demo.foil");
        let build_foil_cmd = FoilCommand::Build {
            target: CliBuildTarget::Foil,
            output: Some(out_foil.clone()),
        };
        execute_cli(build_foil_cmd, &deck_dir).expect("execute build foil");
        assert!(out_foil.is_file());

        let reader = FoilArchiveReader::from_path(&out_foil).expect("verify built foil");
        assert!(reader.get_asset("/index.html").is_some());

        let out_web = deck_dir.join("out-web");
        let build_web_cmd = FoilCommand::Build {
            target: CliBuildTarget::Web,
            output: Some(out_web.clone()),
        };
        execute_cli(build_web_cmd, &deck_dir).expect("execute build web");
        assert!(out_web.join("index.html").is_file());
        assert!(out_web.join("integrity.json").is_file());
    }

    #[test]
    fn test_cli_parses_no_args_and_positional_foil_file() {
        let bare = FoilCli::try_parse_from(["foil"]).expect("parse bare foil");
        assert!(bare.command.is_none());
        assert!(bare.file.is_none());

        let positional =
            FoilCli::try_parse_from(["foil", "presentation.foil"]).expect("parse positional file");
        assert!(positional.command.is_none());
        assert_eq!(
            positional.file.as_deref(),
            Some(std::path::Path::new("presentation.foil"))
        );
    }

    #[test]
    fn test_file_url_to_path_decodes_macos_file_urls_and_rejects_remote_schemes() {
        use super::gui::file_url_to_path;
        use std::path::PathBuf;

        assert_eq!(
            file_url_to_path("file:///Users/francisco/Minha%20Apresentacao.foil"),
            Some(PathBuf::from("/Users/francisco/Minha Apresentacao.foil"))
        );
        assert_eq!(
            file_url_to_path("file://localhost/tmp/deck.foil"),
            Some(PathBuf::from("/tmp/deck.foil"))
        );
        assert_eq!(
            file_url_to_path("/tmp/local.foil"),
            Some(PathBuf::from("/tmp/local.foil"))
        );
        assert_eq!(file_url_to_path("https://example.com/evil.foil"), None);
        assert_eq!(file_url_to_path("   "), None);
    }

    #[test]
    fn test_install_cli_to_dir_creates_and_updates_symlink() {
        use super::gui::install_cli_to_dir;

        let dir = tempdir().expect("tempdir");
        let fake_exe = dir.path().join("foil-bin");
        std::fs::write(&fake_exe, b"binary").expect("write fake exe");
        let bin_dir = dir.path().join("bin");

        let dest = install_cli_to_dir(&fake_exe, &bin_dir).expect("install cli");
        assert!(dest.exists());

        let dest_again = install_cli_to_dir(&fake_exe, &bin_dir).expect("reinstall cli");
        assert_eq!(dest, dest_again);
    }

    #[test]
    fn test_install_skill_to_dir_writes_skill_and_references() {
        use super::gui::install_skill_to_dir;

        let dir = tempdir().expect("tempdir");
        let skills_root = dir.path().join(".agents").join("skills");
        let dest = install_skill_to_dir(&skills_root).expect("install skill");

        assert!(dest.join("SKILL.md").is_file());
        assert!(dest.join("references/design-and-typography.md").is_file());
        assert!(dest.join("references/motion-and-3d.md").is_file());
        assert!(dest.join("references/charts-and-diagrams.md").is_file());
    }
}

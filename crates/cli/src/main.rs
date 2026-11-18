pub mod commands;
pub mod scaffolding;

use clap::Parser;
use commands::{execute_cli, FoilCli};

fn main() {
    let cli = FoilCli::parse();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Err(err) = execute_cli(cli, &cwd) {
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
            FoilCommand::Create {
                template: TemplateKind::ThreeD,
                no_install: true,
                ..
            }
        ));

        let build_web =
            FoilCli::try_parse_from(["foil", "build", "--target", "web"]).expect("parse build web");
        assert!(matches!(
            build_web.command,
            FoilCommand::Build {
                target: CliBuildTarget::Web,
                ..
            }
        ));
    }

    #[test]
    fn test_cli_create_and_build_foil_and_web_targets() {
        let dir = tempdir().expect("tempdir");
        let create_cmd = FoilCli {
            command: FoilCommand::Create {
                name: "demo-deck".to_string(),
                template: TemplateKind::Pitch,
                no_install: true,
            },
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
        let build_foil_cmd = FoilCli {
            command: FoilCommand::Build {
                target: CliBuildTarget::Foil,
                output: Some(out_foil.clone()),
            },
        };
        execute_cli(build_foil_cmd, &deck_dir).expect("execute build foil");
        assert!(out_foil.is_file());

        let reader = FoilArchiveReader::from_path(&out_foil).expect("verify built foil");
        assert!(reader.get_asset("/index.html").is_some());

        let out_web = deck_dir.join("out-web");
        let build_web_cmd = FoilCli {
            command: FoilCommand::Build {
                target: CliBuildTarget::Web,
                output: Some(out_web.clone()),
            },
        };
        execute_cli(build_web_cmd, &deck_dir).expect("execute build web");
        assert!(out_web.join("index.html").is_file());
        assert!(out_web.join("integrity.json").is_file());
    }
}

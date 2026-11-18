# Foil Core (`foil`)

**Foil** é uma engine de apresentações **AI-First** com performance nativa, construída em **Rust** (`tao` + `wry`) para renderizar apresentações em HTML, CSS e Vanilla JS/WebGL (`1920x1080` a 60/120fps) dentro de um sandbox hermético 100% offline.

---

## Arquitetura do Workspace (`core/`)

```text
core/
├── Cargo.toml
└── crates/
    ├── cli/          # Binário `foil` (clap): create, init, dev, build, open, mcp
    ├── packager/     # Scanner estático, sanitização SVG, WebP, SHA-256 e leitor .foil
    ├── player/       # Janela nativa (tao) + WebView (wry) + protocolo foil:// + IPC
    └── mcp-server/   # Servidor MCP (JSON-RPC 2.0 via stdio) para agentes de IA
```

---

## Comandos da CLI (`foil`)

| Comando | Descrição |
| :--- | :--- |
| `foil create <nome>` | Cria um novo projeto de apresentação a partir do template embutido (`--template <minimal\|3d\|pitch>`, `--no-install`). |
| `foil init` | Inicializa uma apresentação no diretório atual. |
| `foil dev` | Inicia o servidor de desenvolvimento Vite (HMR), sobe a ponte local de inspeção e abre o Player nativo (`--headless`, `--browser`, `--port <5173>`). |
| `foil build` | Executa o scanner de segurança, converte imagens para WebP, gera o manifesto SHA-256 (`integrity.json`) e empacota o arquivo `.foil` (`--target <foil\|web>`, `-o <caminho>`). |
| `foil open <arquivo.foil>` | Valida a integridade SHA-256 e abre o pacote `.foil` instantaneamente na WebView nativa via protocolo customizado `foil://localhost/...` sem abrir portas HTTP. |
| `foil mcp` | Inicia o servidor Model Context Protocol (`stdio`) expondo as ferramentas `create_deck`, `upsert_slide`, `install_package`, `inspect_slide` e `package_deck`. |

### Atalhos do Player Nativo (`foil open` / `foil dev`)

* **`→` / `Espaço` / `PageDown`:** Avança para o próximo step ou slide.
* **`←` / `PageUp`:** Retorna ao step ou slide anterior.
* **`F11` / `F`:** Alterna modo tela cheia (Fullscreen).
* **`Esc`:** Sai do modo tela cheia.

---

## Segurança em Profundidade (6 Camadas Anti-Malware)

Pacotes `.foil` de terceiros são tratados como entradas não-confiáveis:

1. **Sandbox WebView & CSP 100% Offline:** Todas as respostas `foil://localhost/...` injetam `Content-Security-Policy` com `connect-src 'none'; object-src 'none'; frame-src 'none'`. Navegações externas, popups (`NewWindowResponse::Deny`), downloads e permissões de hardware (`PermissionResponse::Deny`) são bloqueados na engine.
2. **Blindagem IPC Zero-Privilege:** A ponte `window.ipc.postMessage` aceita exclusivamente o enum `SafeIpcMessage` (`PresenterSync`, `ToggleFullscreen`) com `#[serde(deny_unknown_fields)]`, sem acesso a filesystem ou shell.
3. **Anti-Zip Bomb & Anti-Zip Slip:** Limite estrito de `1.000` arquivos e `250 MB` descompactados, rejeição de caminhos com `..` e allowlist de extensões (`.html`, `.js`, `.css`, `.json`, `.webp`, `.avif`, `.png`, `.jpg`, `.svg`, `.woff2`, `.mp4`, `.glb`).
4. **Sanitização de SVG & Scanner Estático:** Remoção de `<script>`, `<foreignObject>`, `<iframe>`, `on*` e URIs `javascript:` em SVGs; bloqueio de `eval(`, `new Function(`, `child_process`, `node:fs` e scripts `preinstall`/`postinstall`.
5. **Supply Chain Guard:** Instalação de dependências sempre forçada com `bun install --ignore-scripts`.
6. **Manifesto Criptográfico SHA-256 (`integrity.json`):** Todo arquivo empacotado no `.foil` tem seu hash SHA-256 verificado antes de ser carregado em memória pelo `FoilArchiveReader`.

---

## Desenvolvimento, Testes & Build

### Pré-requisitos
* **Rust** `1.85+` (Edition 2024)
* **Bun** `1.1+` (para scaffolding e build front-end de projetos)
* **Linux (Ubuntu/Debian):** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Executar Testes e Linter

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

### Compilar Binário de Produção

```bash
cargo build --release -p foil-cli
```

O binário final estará disponível em `target/release/foil`.

---

## Empacotamento & Releases (`.dmg`, `.deb`, `.msi`)

O workflow [`.github/workflows/ci-release.yml`](./.github/workflows/ci-release.yml) executa os testes nos 3 sistemas operacionais e gera automaticamente os instaladores nativos quando uma **Release** é criada no GitHub (ou uma tag `v*` é enviada):

* **macOS (`.dmg`):** Binário Universal (`aarch64-apple-darwin` + `x86_64-apple-darwin`) empacotado em `Foil.app` com associação da extensão `.foil`.
* **Linux (`.deb`):** Gerado via `cargo-deb` com dependências `libwebkit2gtk-4.1-0` e `libgtk-3-0`.
* **Windows (`.msi`):** Gerado via `cargo-wix` ([`crates/cli/wix/main.wxs`](./crates/cli/wix/main.wxs)) com configuração de `PATH` e associação da extensão `.foil`.

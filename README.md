# Dynamic Lite

Dynamic Island para macOS: uma forma preta "líquida" que vive no notch e vira player do Spotify.

| Estado | Quando | Mostra |
|---|---|---|
| Idle | nada tocando | só o notch, invisível |
| Compact | Spotify tocando | capa à esquerda, equalizer à direita |
| Expanded | mouse sobre a Island | capa, faixa, progresso com seek, prev/play/next, volume |

- Pausado: a Island volta ao tamanho do notch, mas abre no hover pra dar play.
- Mac sem notch: simula um notch no topo central da tela principal.
- Sem ícone no Dock. Ícone de pílula na menu bar com **Iniciar com o macOS** e **Sair**.

## Requisitos

- macOS 12+ (Apple Silicon ou Intel)
- [Xcode Command Line Tools](https://developer.apple.com/xcode/resources/): `xcode-select --install`
- [Rust](https://rustup.rs): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [Bun](https://bun.sh): `curl -fsSL https://bun.sh/install | bash`
- Spotify desktop instalado

## Desenvolvimento

```bash
bun install
bun run tauri dev
```

Hot reload no React; mudanças em `src-tauri/` recompilam e reiniciam o app sozinhas.

## Gerar o app e o DMG

```bash
bun run tauri build
```

Saída:

| Arquivo | Caminho |
|---|---|
| App | `src-tauri/target/release/bundle/macos/Dynamic Lite.app` |
| Instalador | `src-tauri/target/release/bundle/dmg/Dynamic Lite_<versão>_<arch>.dmg` |

Só o `.app` (mais rápido, sem DMG):

```bash
bun run tauri build --bundles app
```

### Universal (Apple Silicon + Intel)

Precisa do Rust via **rustup** (o Rust do Homebrew só tem o target da máquina atual):

```bash
brew uninstall rust            # se instalou pelo Homebrew
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-apple-darwin aarch64-apple-darwin
bun run tauri build --target universal-apple-darwin
```

Sai em `src-tauri/target/universal-apple-darwin/release/bundle/`.

### Versão

Altere `version` em `src-tauri/tauri.conf.json` (vai pro nome do DMG e pro "Sobre" do app).

### Ícone

```bash
bun run tauri icon caminho/para/icone-1024.png
```

Gera todos os tamanhos em `src-tauri/icons/`. Depois rode o build de novo.

## Instalar

1. Feche qualquer `tauri dev` rodando (senão aparecem duas Islands).
2. Abra o `.dmg` e arraste **Dynamic Lite** pra **Aplicativos**, ou:
   ```bash
   cp -R "src-tauri/target/release/bundle/macos/Dynamic Lite.app" /Applications/
   open "/Applications/Dynamic Lite.app"
   ```
3. Aceite o pedido de **Automação → Spotify**.
4. Opcional: menu bar → **Iniciar com o macOS**.

## Distribuir pra outras pessoas

O build local não é assinado. Em outro Mac, o Gatekeeper bloqueia o app baixado. Opções:

- **Sem conta de desenvolvedor**: depois de arrastar pra Aplicativos, a pessoa roda no Terminal
  ```bash
  xattr -cr "/Applications/Dynamic Lite.app"
  ```
  Ou tenta abrir uma vez, vai em Ajustes do Sistema → Privacidade e Segurança, rola até o aviso do Dynamic Lite e clica **Abrir Mesmo Assim**. (No macOS 15+ o truque de botão direito → Abrir não funciona mais.)
- **Com Apple Developer Program** (US$ 99/ano): assine e notarize no build exportando as variáveis antes do `tauri build`:
  ```bash
  export APPLE_SIGNING_IDENTITY="Developer ID Application: Seu Nome (TEAMID)"
  export APPLE_ID="voce@exemplo.com"
  export APPLE_PASSWORD="senha-de-app"   # appleid.apple.com → Senhas de app
  export APPLE_TEAM_ID="TEAMID"
  bun run tauri build
  ```
  Detalhes: [Tauri: macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/).

## Problemas comuns

| Sintoma | Solução |
|---|---|
| Island não mostra a música | Ajustes do Sistema → Privacidade e Segurança → Automação → Dynamic Lite → ative **Spotify** |
| Negou a permissão sem querer | `tccutil reset AppleEvents com.rafael.dynamic-lite` e abra o app de novo |
| Duas Islands na tela | Feche o `tauri dev` ou a cópia instalada |
| `Port 1420 is already in use` no dev | Já tem um `tauri dev` aberto; feche o outro terminal |
| "não é compatível com este Mac" | Mac Intel com build `aarch64`: gere o build Universal |
| "não pode ser aberto" / "está danificado" | App não assinado: `xattr -cr "/Applications/Dynamic Lite.app"` |
| Island na tela errada | Ela fica na tela principal (a que tem a menu bar): Ajustes → Monitores |

Em dev, a permissão de Automação aparece no nome do terminal/VS Code, não do Dynamic Lite.

## Limitações

- Só Spotify. Outros players exigiriam o MediaRemote, bloqueado pra apps de terceiros desde o macOS 15.4 ([ADR 0002](docs/adr/0002-spotify-via-applescript.md)).
- Não dá pra curtir faixas: o AppleScript do Spotify não expõe isso, e a Web API exige app de desenvolvedor + Premium.
- No Compact, a Island pode cobrir itens da menu bar colados no notch.

## Estrutura

```
src-tauri/src/
  lib.rs        setup, comandos, hover + click-through
  macos.rs      AppKit: geometria do notch, nível da janela, mouse, notificações
  layout.rs     tamanho da janela e Hit Region
  spotify.rs    leitura/controle via AppleScript + watcher
  tray.rs       ícone e menu da menu bar
src/
  components/Island.tsx       forma líquida (molas), Ears, estados
  activities/                 Activities plugáveis (v1: now-playing)
```

Vocabulário em [CONTEXT.md](CONTEXT.md), decisões de arquitetura em [docs/adr](docs/adr).

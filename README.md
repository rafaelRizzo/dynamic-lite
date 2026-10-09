# Dynamic Lite

[![Release](https://img.shields.io/github/v/release/rafaelRizzo/dynamic-lite?style=flat-square&label=release&color=2EA44F&labelColor=2EA44F&logo=github&logoColor=white)](https://github.com/rafaelRizzo/dynamic-lite/releases/latest)
![macOS 12+](https://img.shields.io/badge/macOS_12%2B-000000?style=flat-square&logo=apple&logoColor=white)
![Liquid Glass](https://img.shields.io/badge/Liquid_Glass_macOS_26%2B-8E8E93?style=flat-square&logo=apple&logoColor=white)

![Tauri 2](https://img.shields.io/badge/Tauri_2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-CE422B?style=flat-square&logo=rust&logoColor=white)
![React 19](https://img.shields.io/badge/React_19-61DAFB?style=flat-square&logo=react&logoColor=black)
![TypeScript 7](https://img.shields.io/badge/TypeScript_7-3178C6?style=flat-square&logo=typescript&logoColor=white)
![Vite 8](https://img.shields.io/badge/Vite_8-646CFF?style=flat-square&logo=vite&logoColor=white)
![Bun](https://img.shields.io/badge/Bun-000000?style=flat-square&logo=bun&logoColor=white)
![Spotify](https://img.shields.io/badge/Spotify-1DB954?style=flat-square&logo=spotify&logoColor=white)

Dynamic Island para macOS: uma forma preta "líquida" que vive no notch e vira player do Spotify.

**Compact**: tocando, capa à esquerda e equalizer à direita do notch.

![Island compacta no notch, com capa à esquerda e equalizer à direita](docs/screenshots/compact.png)

**Expanded**: mouse em cima, com progresso, controles e volume.

![Island aberta com capa, faixa, progresso, controles e volume](docs/screenshots/expanded.png)

| Estado | Quando | Mostra |
|---|---|---|
| Idle | nada tocando | só o notch, invisível |
| Compact | Spotify tocando | capa à esquerda, equalizer à direita |
| Expanded | mouse sobre a Island | capa, faixa, progresso com seek, prev/play/next, volume |

- Pausado: a Island volta ao tamanho do notch, mas abre no hover pra dar play.
- Mac sem notch: simula um notch no topo central da tela (ou some sem música, configurável).
- Sem ícone no Dock. **Clique direito na Island** ou no ícone da menu bar: **Ajustes…** e **Sair**.

## Ajustes

Clique direito na Island → **Ajustes…** (ou ícone da menu bar). Tudo vale na hora, sem reiniciar.

| Grupo | Ajuste | Opções |
|---|---|---|
| Aparência | Estilo do painel aberto | Preto, Translúcido (opacidade 60-100%), Vidro (Liquid Glass, macOS 26+) |
| | Cor de destaque | Da capa, Branca |
| Comportamento | Abrir com | Passar o mouse (com atraso ajustável), Clique |
| | Toque no trackpad | Clique leve sentido no dedo ao abrir e fechar |
| Tela | Mostrar em | Automática (tela com notch), Principal, ou uma tela pelo nome |
| | Esconder sem música | Só em telas sem notch |
| Sistema | Iniciar com o macOS | |
| | Ícone na menu bar | Sem ele, o acesso é pelo clique direito |

O estado compacto é sempre preto, pra fundir com o notch. No **Vidro**, o painel aberto vira Liquid Glass saindo do notch, com o mesmo formato e animação do Preto ([ADR 0004](docs/adr/0004-liquid-glass-nativo.md)); em macOS sem suporte, cai pro Translúcido.

Ficam salvos em `~/Library/Application Support/com.rafael.dynamic-lite/settings.json`. Pra voltar ao padrão, apague esse arquivo e reabra o app.

## Baixar

Pegue o `.dmg` mais recente em [Releases](https://github.com/rafaelRizzo/dynamic-lite/releases/latest). É universal (Apple Silicon e Intel). Depois siga [Instalar](#instalar).

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

## Releases automáticas

| Evento | Workflow | O que faz |
|---|---|---|
| PR pra `main` | [ci.yml](.github/workflows/ci.yml) | Typecheck, build do frontend, `cargo check` e prévia da próxima versão no resumo do job |
| Push/merge na `main` | [release.yml](.github/workflows/release.yml) | Calcula a versão, builda o `.dmg` universal, cria a tag `vX.Y.Z` e publica a release |

A versão sai dos commits desde a última tag ([Conventional Commits](https://www.conventionalcommits.org/pt-br/)):

| Commit | Exemplo | 0.1.0 vira |
|---|---|---|
| `feat` | `feat: Adicionado volume` | 0.2.0 |
| `fix`, `perf`, `refactor`, `ui` | `fix: Corrigida linha do vidro` | 0.1.1 |
| `!` ou `BREAKING CHANGE` no corpo | `feat!: Trocado formato das Settings` | 1.0.0 |
| `docs`, `chore`, `ci`, `style`, `test`, `build` | `docs: README` | sem release |

A tag é a fonte da verdade: a `main` não recebe commit de versão; o `tauri.conf.json` só é atualizado dentro do build. A lógica está em [next-version.sh](.github/scripts/next-version.sh) (dá pra rodar local pra ver a próxima versão). Também dá pra disparar manualmente em Actions → Release → Run workflow.

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
  macos.rs      AppKit: telas/notch, janela, mouse, haptics, Liquid Glass, notificações
  layout.rs     tamanho da janela e Hit Region
  spotify.rs    leitura/controle via AppleScript + watcher
  menu.rs       menu da menu bar, Context Menu, Settings Window
  settings.rs   Settings: tipos, persistência, aplicação
src/
  components/Island.tsx       forma líquida (molas), Ears, estados
  activities/                 Activities plugáveis (v1: now-playing)
  settings/                   UI da Settings Window
```

Vocabulário em [CONTEXT.md](CONTEXT.md), decisões de arquitetura em [docs/adr](docs/adr).

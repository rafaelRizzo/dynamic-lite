<p align="center"><img src="src-tauri/icons/icon.png" width="128" alt="Ícone do Dynamic Lite"></p>

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
[![Licença MIT](https://img.shields.io/badge/Licen%C3%A7a-MIT-F7DF1E?style=flat-square&logoColor=black)](LICENSE)

Dynamic Island para macOS: uma forma "líquida" que vive no notch e vira player do Spotify.

> Inspirado no **Alcove**. Este é um projeto independente e open source, feito como uma alternativa livre. Não tem relação com o Alcove nem com seus criadores e não tem nenhuma intenção de prejudicá-los.

**Compacta**: tocando, capa à esquerda e equalizer à direita do notch.

![Island compacta no notch, com capa à esquerda e equalizer à direita](docs/screenshots/compact.png)

**Aberta**: mouse em cima, com progresso, controles e volume.

![Island aberta com capa, faixa, progresso, controles e volume](docs/screenshots/expanded.png)

## Instalar

1. Baixe o `.dmg` mais recente em [Releases](https://github.com/rafaelRizzo/dynamic-lite/releases/latest). É universal (Apple Silicon e Intel), macOS 12 ou mais novo.
2. Abra o `.dmg` e arraste **Dynamic Lite** pra **Aplicativos**.
3. O app não é assinado pela Apple. Na primeira vez, rode no Terminal:
   ```bash
   xattr -cr "/Applications/Dynamic Lite.app"
   ```
   Ou tente abrir uma vez, vá em Ajustes do Sistema → Privacidade e Segurança e clique **Abrir Mesmo Assim** no aviso do Dynamic Lite.
4. Abra o app e aceite o pedido de **Automação → Spotify**.
5. Ao dar play, aceite o pedido de **gravação de áudio do sistema** (macOS 14.2+): o equalizer passa a seguir a música. Negando, ele só anima em loop.

Precisa do Spotify desktop instalado.

## Usar

| Estado | Quando | Mostra |
|---|---|---|
| Parada | nada tocando | só o notch, invisível |
| Compacta | Spotify tocando | capa à esquerda, equalizer à direita |
| Aberta | mouse sobre a Island | capa, faixa, progresso, controles, aleatório, repetir e volume |

- **Clique na capa** (aberta): abre a faixa no Spotify.
- **Volume**: o botão ao lado do tempo abre um painel com mute.
- **Repetir**: um clique repete a playlist, outro repete só a faixa.
- **Pausado**: a Island volta ao tamanho do notch, mas abre no hover pra dar play.
- **Mac sem notch**: simula um notch no topo central da tela (ou some sem música, configurável).
- **Clique direito na Island** ou no ícone da menu bar: **Ajustes…** e **Sair**. Não tem ícone no Dock.
- Roda uma instância só: abrir o app de novo traz os Ajustes.

## Ajustes

Clique direito na Island → **Ajustes…**. Tudo vale na hora, sem reiniciar.

| Grupo | Ajuste | Opções |
|---|---|---|
| Aparência | Estilo do painel aberto | Preto, Translúcido (opacidade 60-100%), Vidro (Liquid Glass, macOS 26+), Capa |
| | Cor de destaque | Da capa, Branca |
| Comportamento | Abrir com | Passar o mouse (com atraso ajustável), Clique |
| | Toque no trackpad | Clique leve sentido no dedo ao abrir e fechar |
| Tela | Mostrar em | Automática (tela com notch), Principal, ou uma tela pelo nome |
| | Mesas | Todas, ou só a atual |
| | Esconder no Mission Control | Não cobre a barra de mesas no topo |
| | Esconder sem música | Só em telas sem notch |
| Atualizações | Verificar automaticamente | Ao abrir e uma vez por dia; botão "Verificar agora" |
| Sistema | Iniciar com o macOS | |
| | Ícone na menu bar | Sem ele, o acesso é pelo clique direito |

Sobre os estilos:

- A Island compacta é preta, pra fundir com o notch (exceto no estilo Capa).
- **Vidro**: o painel aberto vira Liquid Glass saindo do notch. Em macOS sem suporte, cai pro Translúcido.
- **Capa**: a Island compacta e o painel aberto ficam na cor da capa da música, com texto e controles claros ou escuros conforme o contraste.

Os ajustes ficam em `~/Library/Application Support/com.rafael.dynamic-lite/settings.json`. Pra voltar ao padrão, apague o arquivo e reabra o app.

## Atualizações

O app procura versão nova ao abrir, uma vez por dia e sempre que você abre os Ajustes. Quando encontra:

- Os Ajustes mostram **Atualizar pra X.Y.Z** no topo.
- O clique direito e o menu da menu bar ganham **Instalar atualização e reabrir**.
- Sem música tocando, a Island mostra um aviso com **Depois** e **Instalar e reabrir**.

A instalação baixa, substitui o app e reabre sozinha.

## Gerar o app você mesmo

Requisitos:

- [Xcode Command Line Tools](https://developer.apple.com/xcode/resources/): `xcode-select --install`
- [Rust](https://rustup.rs): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [Bun](https://bun.sh): `curl -fsSL https://bun.sh/install | bash`

Build:

```bash
git clone https://github.com/rafaelRizzo/dynamic-lite.git
cd dynamic-lite
bun install
bun run tauri build
```

| Arquivo | Caminho |
|---|---|
| App | `src-tauri/target/release/bundle/macos/Dynamic Lite.app` |
| Instalador | `src-tauri/target/release/bundle/dmg/Dynamic Lite_<versão>_<arch>.dmg` |

Instale abrindo o `.dmg` ou copiando o app direto:

```bash
cp -R "src-tauri/target/release/bundle/macos/Dynamic Lite.app" /Applications/
open "/Applications/Dynamic Lite.app"
```

O build sai pra arquitetura do seu Mac. Pra gerar um universal (Apple Silicon + Intel):

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
bun run tauri build --target universal-apple-darwin
```

Sai em `src-tauri/target/universal-apple-darwin/release/bundle/`. Precisa do Rust instalado pelo rustup; o do Homebrew só tem a arquitetura da máquina.

## Problemas comuns

| Sintoma | Solução |
|---|---|
| Island não mostra a música | Ajustes do Sistema → Privacidade e Segurança → Automação → Dynamic Lite → ative **Spotify** |
| Negou a permissão sem querer | `tccutil reset AppleEvents com.rafael.dynamic-lite` e abra o app de novo |
| Equalizer não segue a música | Ajustes do Sistema → Privacidade e Segurança → Gravação de Tela e Áudio do Sistema → ative **Dynamic Lite** (ou `tccutil reset AudioCapture com.rafael.dynamic-lite`) |
| "não pode ser aberto" / "está danificado" | `xattr -cr "/Applications/Dynamic Lite.app"` |
| "não é compatível com este Mac" | Mac Intel com build só Apple Silicon: baixe o `.dmg` das Releases ou gere o universal |
| Island na tela errada | Ajustes → Tela → **Mostrar em** |
| Island cobrindo a barra de mesas | Ajustes → Tela → **Esconder no Mission Control** |

## Limitações

- Só Spotify. Outros players exigiriam uma API do sistema bloqueada pra apps de terceiros desde o macOS 15.4.
- Não dá pra curtir faixas: o Spotify não expõe isso pro app.
- Na Island compacta, ela pode cobrir itens da menu bar colados no notch.

## Licença

[MIT](LICENSE). Pode usar, modificar e distribuir, inclusive comercialmente, mantendo o aviso de copyright.

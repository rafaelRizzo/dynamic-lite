#!/usr/bin/env bash
# Tauri CLI com a versão do app vinda da maior tag git (vX.Y.Z), sem editar arquivos.
# Maior tag do repo, não a mais próxima do HEAD: as tags ficam nos merges da main, que a dev não contém.
# APP_VERSION no ambiente tem prioridade: o CI usa pra lançar a versão nova antes da tag existir.
# Sem tag e sem APP_VERSION, vale a versão do tauri.conf.json.
#
# Os artefatos de atualização automática precisam da chave privada: no build local usa
# ~/.tauri/dynamic-lite.key (+ .password) se existir; senão desliga esses artefatos.
set -euo pipefail

KEY="$HOME/.tauri/dynamic-lite.key"
version="${APP_VERSION:-$(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname 2>/dev/null | head -n1 | sed 's/^v//' || true)}"

case "${1:-}" in
  dev | build)
    cmd="$1"
    shift
    overrides=()
    if [[ -n "$version" ]]; then
      echo "Dynamic Lite v$version" >&2
      overrides+=("\"version\":\"$version\"")
    fi
    if [[ "$cmd" == build && -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
      if [[ -f "$KEY" ]]; then
        export TAURI_SIGNING_PRIVATE_KEY="$KEY"
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat "$KEY.password" 2>/dev/null || true)"
        export TAURI_SIGNING_PRIVATE_KEY_PASSWORD
      else
        echo "Sem $KEY: build sem artefatos de atualização automática" >&2
        overrides+=('"bundle":{"createUpdaterArtifacts":false}')
      fi
    fi
    if ((${#overrides[@]})); then
      exec tauri "$cmd" --config "{$(IFS=,; echo "${overrides[*]}")}" "$@"
    fi
    exec tauri "$cmd" "$@"
    ;;
esac
exec tauri "$@"

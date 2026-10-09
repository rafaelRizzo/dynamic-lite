#!/usr/bin/env bash
# Tauri CLI com a versão do app vinda da última tag git (vX.Y.Z), sem editar arquivos.
# APP_VERSION no ambiente tem prioridade: o CI usa pra lançar a versão nova antes da tag existir.
# Sem tag e sem APP_VERSION, vale a versão do tauri.conf.json.
set -euo pipefail

version="${APP_VERSION:-$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null | sed 's/^v//' || true)}"

case "${1:-}" in
  dev | build)
    if [[ -n "$version" ]]; then
      cmd="$1"
      shift
      echo "Dynamic Lite v$version" >&2
      exec tauri "$cmd" --config "{\"version\":\"$version\"}" "$@"
    fi
    ;;
esac
exec tauri "$@"

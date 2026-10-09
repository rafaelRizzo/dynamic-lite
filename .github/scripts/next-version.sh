#!/usr/bin/env bash
# Calcula a próxima versão a partir dos Conventional Commits desde a última tag v*.
#   feat              -> minor
#   fix|perf|refactor|ui -> patch
#   tipo! / BREAKING CHANGE -> major
#   demais (docs, chore, ci, style, test, build) -> sem release
# Sem tag ainda: lança a versão do tauri.conf.json como primeira release.
# Saída (formato $GITHUB_OUTPUT): release=true|false, version=X.Y.Z, notes=<markdown>
set -euo pipefail

CONF="src-tauri/tauri.conf.json"
last_tag=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || true)

if [[ -z "$last_tag" ]]; then
  range="HEAD"
  version=$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$CONF")
  bump="initial"
else
  range="$last_tag..HEAD"
  bump="none"
fi

feats=() fixes=() others=() breaking=()
while IFS=$'\x1f' read -r subject body; do
  [[ -z "$subject" ]] && continue
  if [[ "$subject" =~ ^([a-z]+)(\([^\)]*\))?(!)?:\ (.+)$ ]]; then
    type="${BASH_REMATCH[1]}" bang="${BASH_REMATCH[3]}" desc="${BASH_REMATCH[4]}"
  else
    continue # merge commits e afins
  fi
  if [[ -n "$bang" || "$body" == *"BREAKING CHANGE"* ]]; then
    breaking+=("$desc")
    [[ "$bump" != "initial" ]] && bump="major"
  fi
  case "$type" in
    feat) feats+=("$desc"); [[ "$bump" == "none" || "$bump" == "patch" ]] && bump="minor" ;;
    fix|perf) fixes+=("$desc"); [[ "$bump" == "none" ]] && bump="patch" ;;
    refactor|ui) others+=("$desc"); [[ "$bump" == "none" ]] && bump="patch" ;;
  esac
done < <(git log "$range" --no-merges --format='%s%x1f%b%x1e' | tr '\n' ' ' | tr '\036' '\n')

if [[ "$bump" == "none" ]]; then
  echo "release=false"
  exit 0
fi

if [[ "$bump" != "initial" ]]; then
  IFS=. read -r major minor patch <<< "${last_tag#v}"
  case "$bump" in
    major) version="$((major + 1)).0.0" ;;
    minor) version="$major.$((minor + 1)).0" ;;
    patch) version="$major.$minor.$((patch + 1))" ;;
  esac
fi

section() {
  local title="$1"; shift
  (($#)) || return 0
  printf '### %s\n\n' "$title"
  printf -- '- %s\n' "$@"
  printf '\n'
}

notes=$(
  section "⚠️ Mudanças incompatíveis" ${breaking[@]+"${breaking[@]}"}
  section "Novidades" ${feats[@]+"${feats[@]}"}
  section "Correções e desempenho" ${fixes[@]+"${fixes[@]}"}
  section "Melhorias" ${others[@]+"${others[@]}"}
  cat <<'MD'
### Instalação

1. Baixe o `.dmg` abaixo (universal: Apple Silicon e Intel), abra e arraste **Dynamic Lite** pra **Aplicativos**.
2. O app não é assinado pela Apple. Na primeira vez, rode no Terminal:
   ```bash
   xattr -cr "/Applications/Dynamic Lite.app"
   ```
3. Abra o app e aceite a permissão de **Automação → Spotify**.
MD
)

echo "release=true"
echo "version=$version"
echo "notes<<__NOTES__"
echo "$notes"
echo "__NOTES__"

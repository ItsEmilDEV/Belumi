#!/usr/bin/env bash
set -euo pipefail
a=${1:?jar 26.2}; b=${2:?jar 26.3}; out=${3:?output dir}
mkdir -p "$out"
packages=(
  network/protocol/game network/protocol/common network/protocol/configuration
  network/protocol/login network/protocol/status network/protocol/cookie
  network/syncher core/particles core/component world/item/component
  world/item network/chat world/entity/ai/attributes
)

norm() { sed -E 's/#[0-9]+(,  [0-9]+)?//g; s/^ *[0-9]+: //'; }

classes() {
  unzip -l "$1" | awk '{print $4}' | while read -r f; do
    for p in "${packages[@]}"; do
      [[ $f == net/minecraft/$p/*.class && $f != net/minecraft/$p/*/* ]] && echo "${f%.class}" && break
    done
  done | sort -u
}

compare() {
  local c=${1//\//.} name
  name=$(basename "$1")
  local ca cb sa sb
  ca=$(javap -p -c -constants -classpath "$a" "$c" 2>/dev/null | norm || true)
  cb=$(javap -p -c -constants -classpath "$b" "$c" 2>/dev/null | norm || true)
  [[ $ca == "$cb" ]] && return
  sa=$(javap -p -constants -classpath "$a" "$c" 2>/dev/null | norm || true)
  sb=$(javap -p -constants -classpath "$b" "$c" 2>/dev/null | norm || true)
  diff <(echo "$ca") <(echo "$cb") > "$out/${name}.diff" || true
  if [[ $sa == "$sb" ]]; then echo "CODE  $1"; else echo "SHAPE $1"; fi
}
export -f compare norm
export a b out

{
  comm -13 <(classes "$a") <(classes "$b") | sed 's/^/ADDED /'
  comm -23 <(classes "$a") <(classes "$b") | sed 's/^/REMOVED /'
  classes "$a" | comm -12 - <(classes "$b") | xargs -P "$(nproc)" -I{} bash -c 'compare {}'
} | sort > "$out/report.txt"

echo "report: $out/report.txt"
awk '{c[$1]++} END {for (k in c) printf "%s %d\n", k, c[k]}' "$out/report.txt" | sort

#!/bin/sh
set -eu

if [ "$#" -ne 3 ]; then
  echo "usage: write-checksums.sh <version> <owner/repo> <assets dir>" >&2
  exit 2
fi
version=$1
repository=$2
dir=$3

hash() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  else
    shasum -a 256 "$1" | cut -d ' ' -f 1
  fi
}

printf '{\n  "version": "%s",\n  "repository": "%s",\n  "binaries": {' "$version" "$repository"
separator=""
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-msvc; do
  file="$dir/foxmorrow-$target"
  if [ "$target" = x86_64-pc-windows-msvc ]; then
    file="$file.exe"
  fi
  if [ -f "$file" ]; then
    printf '%s\n    "%s": "%s"' "$separator" "$target" "$(hash "$file")"
    separator=","
  fi
done
if [ -n "$separator" ]; then
  printf '\n  '
fi
printf '}\n}\n'

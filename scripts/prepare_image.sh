#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_DIR="${IMAGE_DIR:-${ROOT_DIR}/image}"

# 镜像内容列表：原路径:目标路径（目标路径相对于 IMAGE_DIR）
# 原路径支持 shell 通配符，例如 ${ROOT_DIR}/path/to/dir/*
COPY_LIST=(
	"${ROOT_DIR}/kernel/Cargo.lock:Cargo.lock"
	"${ROOT_DIR}/user/target/hello:hello"
)

copy_entry() {
	local src_spec="$1"
	local dst_spec="$2"

	local -a matches=()
	shopt -s nullglob
	if [[ "$src_spec" == *[*?[]* ]]; then
		matches=($src_spec)
	else
		if [[ ! -e "$src_spec" ]]; then
			echo "error: source not found: $src_spec" >&2
			exit 1
		fi
		matches=("$src_spec")
	fi
	shopt -u nullglob

	if ((${#matches[@]} == 0)); then
		echo "error: no match for: $src_spec" >&2
		exit 1
	fi

	local dst="${IMAGE_DIR}/${dst_spec}"
	local copy_as_dir=0

	if ((${#matches[@]} > 1)); then
		copy_as_dir=1
	elif [[ "$dst_spec" == */ ]]; then
		copy_as_dir=1
	elif [[ -d "$dst" ]]; then
		copy_as_dir=1
	fi

	if ((copy_as_dir == 1)); then
		mkdir -p "$dst"
		for f in "${matches[@]}"; do
			cp -a "$f" "$dst/"
		done
	else
		mkdir -p "$(dirname "$dst")"
		cp -a "${matches[0]}" "$dst"
	fi
}

mkdir -p "$IMAGE_DIR"
rm -rf "${IMAGE_DIR:?}"/*

for entry in "${COPY_LIST[@]}"; do
	[[ -z "$entry" || "$entry" == \#* ]] && continue
	src="${entry%%:*}"
	dst="${entry#*:}"
	if [[ -z "$src" || -z "$dst" || "$src" == "$entry" ]]; then
		echo "error: invalid copy list entry: $entry" >&2
		exit 1
	fi
	copy_entry "$src" "$dst"
done

echo "prepared image contents in ${IMAGE_DIR}"

#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
publish_dir="${repo_root}/../gh-pages"
dist_dir="${repo_root}/target/trunk-dist"
public_url="${TRUNK_PUBLIC_URL:-/MiiCut/}"

if [[ ! -d "${repo_root}/.git" ]]; then
  echo "Not in a git repo: ${repo_root}" >&2
  exit 1
fi

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
  if command -v trunk >/dev/null 2>&1; then
    trunk build --release --public-url "${public_url}"
  else
    echo "trunk not found; skipping build." >&2
  fi
fi

if [[ ! -d "${dist_dir}" ]]; then
  echo "Build output missing: ${dist_dir}" >&2
  echo "Run: trunk build --release --public-url \"${public_url}\"" >&2
  exit 1
fi

cd "${repo_root}"
git worktree prune --expire=now
git worktree remove --force "${publish_dir}" 2>/dev/null || true

if [[ -d "${publish_dir}" ]]; then
  rm -rf "${publish_dir}"
fi

if git show-ref --verify --quiet refs/heads/gh-pages; then
  git worktree add -f "${publish_dir}" gh-pages
else
  git worktree add -b gh-pages "${publish_dir}"
fi

rsync -av --delete --checksum --exclude ".git" "${dist_dir}/" "${publish_dir}/"

git -C "${publish_dir}" add -A
if git -C "${publish_dir}" diff --cached --quiet; then
  echo "No changes to publish."
  exit 0
fi

git -C "${publish_dir}" commit -m "publish $(date +%Y-%m-%d\ %H:%M:%S)"
git -C "${publish_dir}" push origin gh-pages

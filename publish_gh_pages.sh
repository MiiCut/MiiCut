#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
publish_dir="${repo_root}/../gh-pages"
dist_dir="${repo_root}/target/trunk-dist"

if [[ ! -d "${repo_root}/.git" ]]; then
  echo "Not in a git repo: ${repo_root}" >&2
  exit 1
fi

if [[ ! -d "${dist_dir}" ]]; then
  echo "Build output missing: ${dist_dir}" >&2
  echo "Run your build first (trunk build)." >&2
  exit 1
fi

cd "${repo_root}"
git worktree prune

if [[ -d "${publish_dir}" ]]; then
  rm -rf "${publish_dir}"
fi

if git show-ref --verify --quiet refs/heads/gh-pages; then
  git worktree add "${publish_dir}" gh-pages
else
  git worktree add -b gh-pages "${publish_dir}"
fi

rsync -av --delete --exclude ".git" "${dist_dir}/" "${publish_dir}/"

git -C "${publish_dir}" add -A
if git -C "${publish_dir}" diff --cached --quiet; then
  echo "No changes to publish."
  exit 0
fi

git -C "${publish_dir}" commit -m "publish $(date +%Y-%m-%d\ %H:%M:%S)"
git -C "${publish_dir}" push origin gh-pages

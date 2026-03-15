#!/usr/bin/env python3
"""
Deploy MiiCut to grblHAL embedded web server.

The grblHAL RP2040 networking plugin HTTP server only serves index.html(.gz)
from the SD card. This script bundles ALL assets (CSS, fonts, JS, WASM) into a
single self-contained index.html, compresses it, and uploads it as index.html.gz.

Usage:
    python3 deploy_to_grblhal.py [--host 192.168.1.36] [--port 21]
    python3 deploy_to_grblhal.py --clean     # delete www/ contents first
    python3 deploy_to_grblhal.py --list      # list files on server
"""

import argparse
import base64
import ftplib
import gzip
import os
import re
import sys
import tempfile
from pathlib import Path

DIST_DIR = Path(__file__).parent / "target" / "trunk-dist"
FTP_WEB_DIR = "www"


# ---------------------------------------------------------------------------
# FTP helpers
# ---------------------------------------------------------------------------


def ftp_makedirs_rel(ftp: ftplib.FTP, rel_path: str) -> None:
    parts = rel_path.strip("/").split("/")
    current = ""
    for part in parts:
        current = f"{current}/{part}" if current else part
        try:
            ftp.mkd(current)
        except ftplib.error_perm as e:
            if not str(e).startswith("550"):
                raise


def ftp_delete_all(ftp: ftplib.FTP) -> None:
    entries: list[str] = []
    ftp.retrlines("LIST", entries.append)
    for entry in entries:
        parts: list[str] = entry.split(None, 8)
        if len(parts) < 9:
            continue
        name: str = parts[-1]
        is_dir: bool = entry.startswith("d")
        if is_dir:
            ftp.cwd(name)
            ftp_delete_all(ftp)
            ftp.cwd("..")
            try:
                ftp.rmd(name)
            except ftplib.error_perm:
                pass
        else:
            try:
                ftp.delete(name)
            except ftplib.error_perm:
                pass


def ftp_list_rel(ftp: ftplib.FTP, indent: int = 0) -> None:
    entries: list[str] = []
    ftp.retrlines("LIST", entries.append)
    for entry in entries:
        parts: list[str] = entry.split(None, 8)
        if len(parts) < 9:
            continue
        name: str = parts[-1]
        size: str = parts[4]
        is_dir: bool = entry.startswith("d")
        prefix = "  " * indent
        if is_dir:
            print(f"{prefix}[dir]  {name}/")
            ftp.cwd(name)
            ftp_list_rel(ftp, indent + 1)
            ftp.cwd("..")
        else:
            kb = int(size) / 1024 if size.isdigit() else 0
            print(f"{prefix}       {name}  ({kb:.1f} KB)")


def connect_and_cwd(host: str, port: int) -> ftplib.FTP:
    ftp = ftplib.FTP()
    ftp.connect(host, port, timeout=30)
    ftp.login()
    ftp.set_pasv(True)
    print(f"Connected: {ftp.getwelcome()}")
    print(f"FTP root:  {ftp.pwd()}")
    try:
        ftp.cwd(FTP_WEB_DIR)
    except ftplib.error_perm:
        ftp.mkd(FTP_WEB_DIR)
        ftp.cwd(FTP_WEB_DIR)
    print(f"CWD:       {ftp.pwd()}\n")
    return ftp


# ---------------------------------------------------------------------------
# Bundler: everything → single index.html
# ---------------------------------------------------------------------------


def _embed_url(match: re.Match[str], base_dir: Path) -> str:
    """Replace url('...') with url('data:mime;base64,...') for local assets."""
    raw: str = match.group(1).strip("'\" ")
    if raw.startswith("data:") or raw.startswith("http") or raw.startswith("//"):
        return match.group(0)
    # Strip query string / fragment before resolving the file path
    raw_path = raw.split("?")[0].split("#")[0]
    asset_path = (base_dir / raw_path).resolve()
    if not asset_path.exists():
        return match.group(0)
    b64 = base64.b64encode(asset_path.read_bytes()).decode()
    ext = asset_path.suffix.lstrip(".").lower()
    mime = {
        "woff2": "font/woff2",
        "woff": "font/woff",
        "ttf": "font/ttf",
        "otf": "font/otf",
        "svg": "image/svg+xml",
        "png": "image/png",
        "jpg": "image/jpeg",
        "jpeg": "image/jpeg",
    }.get(ext, "application/octet-stream")
    return f"url('data:{mime};base64,{b64}')"


def bundle_single_html(dist_dir: Path) -> bytes:
    """
    Bundle CSS (with embedded fonts/icons), JS, and WASM into one HTML file.

    The grblHAL HTTP server only serves index.html.gz from SD card, so all
    assets must live inside that single file.
    """
    # Locate files
    wasm_files = list(dist_dir.glob("*.wasm"))
    js_files = [f for f in dist_dir.glob("*.js") if not f.name.endswith(".map")]
    css_root_files = list(dist_dir.glob("styles-*.css"))
    urbanist_css = dist_dir / "assets" / "urbanist" / "urbanist.css"
    html_src = dist_dir / "index.html"

    if not wasm_files:
        raise FileNotFoundError("No .wasm file found in trunk-dist/")
    if not js_files:
        raise FileNotFoundError("No .js file found in trunk-dist/")
    if not html_src.exists():
        raise FileNotFoundError("index.html not found in trunk-dist/")

    wasm_path = wasm_files[0]
    js_path = js_files[0]

    # 1. WASM → base64
    wasm_kb = wasm_path.stat().st_size / 1024
    print(f"  Encoding WASM  ({wasm_kb:.0f} KB → base64)...", end=" ", flush=True)
    wasm_b64 = base64.b64encode(wasm_path.read_bytes()).decode()
    print(f"{len(wasm_b64) / 1024:.0f} KB")

    # 2. JS content — escape </ so the HTML parser never sees </script> inside
    js_kb = js_path.stat().st_size / 1024
    print(f"  Reading JS     ({js_kb:.0f} KB)")
    js_content = js_path.read_text(encoding="utf-8").replace("</", "<\\/")

    # 3. CSS with all url() assets (fonts, SVG icons) embedded as data URIs
    css_parts: list[str] = []

    if urbanist_css.exists():
        ucss = urbanist_css.read_text(encoding="utf-8")
        ucss = re.sub(
            r"url\(([^)]+)\)", lambda m: _embed_url(m, urbanist_css.parent), ucss
        )
        css_parts.append(ucss)

    for css_path in css_root_files:
        css_kb = css_path.stat().st_size / 1024
        print(f"  Embedding CSS  ({css_kb:.0f} KB, fonts+icons inlined)")
        css = css_path.read_text(encoding="utf-8")
        css = re.sub(r"url\(([^)]+)\)", lambda m: _embed_url(m, dist_dir), css)
        css_parts.append(css)

    combined_css = "\n".join(css_parts)

    # 4. Build bundled module script.
    # Find the actual internal name of the default-exported init function.
    m = re.search(r"export\s+default\s+(\w+)", js_content)
    init_fn = m.group(1) if m else "init"
    print(f"  Init function  '{init_fn}'")

    # Embed SVG/PNG assets loaded dynamically by Rust (e.g. assets/icon-*.svg?v=N).
    # A MutationObserver intercepts img.src and replaces with data URIs.
    asset_dir = dist_dir / "assets"
    icon_entries: list[str] = []
    for asset in sorted(asset_dir.rglob("*")):
        if asset.is_file() and asset.suffix.lower() in {".svg", ".png"}:
            rel = asset.relative_to(dist_dir).as_posix()  # e.g. "assets/icon-note.svg"
            b64 = base64.b64encode(asset.read_bytes()).decode()
            ext = asset.suffix.lstrip(".").lower()
            mime = "image/svg+xml" if ext == "svg" else "image/png"
            icon_entries.append(f'  "{rel}": "data:{mime};base64,{b64}"')
    icons_map = "{\n" + ",\n".join(icon_entries) + "\n}"
    print(f"  SVG/PNG assets {len(icon_entries)} files embedded for MutationObserver")

    icon_observer_js = f"""
// ── dynamic asset interception (icons loaded by Rust at runtime) ────────────
// Override HTMLImageElement.src setter so data URIs are substituted BEFORE
// the browser initiates any fetch request (eliminates 404s entirely).
(function() {{
  const _assets = {icons_map};
  const _desc = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, 'src');
  Object.defineProperty(HTMLImageElement.prototype, 'src', {{
    set(value) {{
      const key = (value || '').split('?')[0];
      _desc.set.call(this, _assets[key] || value);
    }},
    get() {{ return _desc.get.call(this); }},
    configurable: true,
  }});
}})();
"""

    bundled_script = (
        js_content
        + icon_observer_js
        + f"""
// ── embedded WASM (generated by deploy_to_grblhal.py) ──────────────────────
(async () => {{
  const _b64 = '{wasm_b64}';
  const _bin = atob(_b64);
  const _buf = new Uint8Array(_bin.length);
  for (let i = 0; i < _bin.length; i++) _buf[i] = _bin.charCodeAt(i);
  const wasm = await {init_fn}({{ module_or_path: _buf.buffer }});
  window.wasmBindings = {{}};
  dispatchEvent(new CustomEvent('TrunkApplicationStarted', {{ detail: {{ wasm }} }}));
}})();
"""
    )

    # 5. Patch HTML
    html = html_src.read_text(encoding="utf-8")

    # Remove external stylesheet links
    html = re.sub(
        r'<link[^>]+rel=["\']stylesheet["\'][^>]*/?\s*>', "", html, flags=re.IGNORECASE
    )

    # Remove modulepreload / preload links
    html = re.sub(
        r'<link[^>]+rel=["\'](?:modulepreload|preload)["\'][^>]*/?\s*>',
        "",
        html,
        flags=re.IGNORECASE,
    )

    # Replace the inline <script type="module">…</script> with the bundle.
    # Use a lambda so re.sub does NOT interpret \n, \t, etc. in the replacement.
    replacement = (
        f"<style>\n{combined_css}\n</style>\n"
        f'<script type="module">\n{bundled_script}\n</script>'
    )
    html = re.sub(
        r'<script\s+type=["\']module["\'][^>]*>.*?</script>',
        lambda _: replacement,
        html,
        flags=re.DOTALL | re.IGNORECASE,
    )

    result = html.encode("utf-8")
    print(f"  HTML bundle    {len(result) / 1024:.0f} KB uncompressed")
    return result


def compress_bytes(data: bytes) -> Path:
    tmp = tempfile.NamedTemporaryFile(delete=False, suffix=".gz")
    with gzip.open(tmp.name, "wb", compresslevel=9) as f:
        f.write(data)
    return Path(tmp.name)


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------


def cmd_list(host: str, port: int) -> None:
    with connect_and_cwd(host, port) as ftp:
        print(f"Contents of {FTP_WEB_DIR}/ on {host}:\n")
        ftp_list_rel(ftp)


def cmd_deploy(host: str, port: int, clean: bool) -> None:
    if not DIST_DIR.exists():
        print(f"ERROR: {DIST_DIR} not found — run 'trunk build' first.")
        sys.exit(1)

    print("Bundling assets into single index.html …")
    html_bytes = bundle_single_html(DIST_DIR)

    gz_path = compress_bytes(html_bytes)
    gz_kb = gz_path.stat().st_size / 1024
    print(f"  Compressed     {gz_kb:.0f} KB  (index.html.gz)\n")

    try:
        with connect_and_cwd(host, port) as ftp:
            if clean:
                print("Cleaning www/ …")
                ftp_delete_all(ftp)
                print("Done.\n")

            print("Uploading index.html.gz …", end=" ", flush=True)
            with open(gz_path, "rb") as f:
                ftp.storbinary("STOR index.html.gz", f)
            print("✓")
    finally:
        os.unlink(gz_path)

    print("\nDone.")


# ---------------------------------------------------------------------------

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Bundle and deploy MiiCut to grblHAL FTP."
    )
    parser.add_argument("--host", default="192.168.1.36")
    parser.add_argument("--port", type=int, default=21)
    parser.add_argument("--list", action="store_true", help="List files on server")
    parser.add_argument(
        "--clean", action="store_true", help="Delete www/ contents before deploying"
    )
    args = parser.parse_args()

    if args.list:
        cmd_list(args.host, args.port)
    else:
        cmd_deploy(args.host, args.port, clean=args.clean)

#!/usr/bin/env python3

import argparse
import functools
import http.server
import pathlib
import shutil
import subprocess
import sys
import threading
import webbrowser


ROOT = pathlib.Path(__file__).resolve().parent


class PreviewHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
    }

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


def main():
    parser = argparse.ArgumentParser(
        description="Build WASM and open a local browser preview."
    )
    parser.add_argument(
        "--release",
        action="store_true",
        help="Build an optimized release version.",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8000,
        help="HTTP port; use 0 to choose a free port automatically.",
    )
    parser.add_argument(
        "--out-dir",
        default="pkg",
        help="WASM output directory matching the import in index.html.",
    )
    parser.add_argument(
        "--no-open",
        action="store_true",
        help="Do not open the browser automatically.",
    )
    args = parser.parse_args()

    for filename in ("Cargo.toml", "index.html"):
        if not (ROOT / filename).is_file():
            parser.error(f"Missing {filename} next to preview.py")

    wasm_pack = shutil.which("wasm-pack")
    if wasm_pack is None:
        sys.exit(
            "wasm-pack was not found.\n"
            "Install it with: cargo install wasm-pack --locked"
        )

    handler = functools.partial(PreviewHandler, directory=str(ROOT))

    try:
        server = http.server.ThreadingHTTPServer(
            ("127.0.0.1", args.port),
            handler,
        )
    except OSError as error:
        sys.exit(
            f"Cannot start the preview server: {error}\n"
            "Try another port or use --port 0."
        )

    with server:
        command = [
            wasm_pack,
            "build",
            "--target",
            "web",
            "--out-dir",
            args.out_dir,
            "--release" if args.release else "--dev",
        ]

        print("\nBuilding:", " ".join(command), flush=True)

        try:
            subprocess.run(command, cwd=ROOT, check=True)
        except subprocess.CalledProcessError as error:
            sys.exit(error.returncode)

        port = server.server_address[1]
        url = f"http://127.0.0.1:{port}/"

        print(f"\nPreview: {url}")
        print("Stop the server with Ctrl+C.\n", flush=True)

        if not args.no_open:
            def open_browser():
                if not webbrowser.open(url):
                    print(f"Open the preview manually: {url}", flush=True)

            threading.Thread(target=open_browser, daemon=True).start()

        try:
            server.serve_forever()
        except KeyboardInterrupt:
            print("\nPreview stopped.")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nCancelled.")
        sys.exit(130)

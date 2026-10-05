"""Fail if Git's upload set contains local game data or machine-specific paths."""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = {".iso", ".ciso", ".gcm", ".rvz", ".dol", ".prs", ".gvm", ".bin", ".png", ".wav", ".exe", ".dll", ".zip", ".7z", ".log"}

def main():
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
    paths = [p for p in paths if p]
    if not paths:
        raise SystemExit("Empty upload set: add the intended source files to Git first")
    for name in paths:
        path = Path(name)
        if path.suffix.lower() in FORBIDDEN or any(p in {".local", "target", "source", "movement-verification"} for p in path.parts):
            raise SystemExit("Local/private data in upload: " + name)
        if name.startswith("playable/assets/") and name != "playable/assets/README.md":
            raise SystemExit("Extracted asset in upload: " + name)
        content = (ROOT / name).read_bytes()
        if len(content) > 2 * 1024 * 1024:
            raise SystemExit("Unexpected large upload file: " + name)
        try:
            text = content.decode("utf-8-sig")
        except UnicodeError:
            raise SystemExit("Unexpected binary upload file: " + name)
        if re.search(r"[CD]:[\\/](?:Users|CodexBuilds)[\\/]", text, re.I):
            raise SystemExit("Machine-specific path in upload: " + name)
    print(f"UPLOAD_AUDIT_OK {len(paths)} text source/documentation files; no game data or binaries")

if __name__ == "__main__":
    main()

"""Prepare ignored, local data from the supported GameCube release. No downloads."""
import argparse
import hashlib
import json
import struct
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOL_SHA = "a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e"
ARCHIVES = ("plcommtn.prs", "sonicmtn.prs", "sonicmdl.prs")
MAX_READ = 32 * 1024 * 1024

def word(data, offset):
    return struct.unpack_from(">I", data, offset)[0]

class Image:
    def __init__(self, path):
        self.file = path.open("rb")
        self.size = path.stat().st_size
        header = self.file.read(0x8000)
        self.blocks = None
        if header[:4] == b"CISO":
            if len(header) != 0x8000:
                raise ValueError("Truncated CISO header")
            self.block_size = struct.unpack_from("<I", header, 4)[0]
            if not 0 < self.block_size <= 16 * 1024 * 1024:
                raise ValueError("Invalid CISO block size")
            physical = 0x8000
            self.blocks = []
            for present in header[8:]:
                if present not in (0, 1):
                    raise ValueError("Invalid CISO map")
                self.blocks.append(physical if present else None)
                physical += self.block_size if present else 0
            if physical > self.size:
                raise ValueError("Truncated CISO image")

    def close(self):
        self.file.close()

    def read(self, offset, size):
        limit = self.size if self.blocks is None else len(self.blocks) * self.block_size
        if offset < 0 or size < 0 or size > MAX_READ or offset + size > limit:
            raise ValueError("Read outside game image")
        if self.blocks is None:
            self.file.seek(offset)
            data = self.file.read(size)
            if len(data) != size:
                raise ValueError("Truncated game image")
            return data
        output = bytearray()
        while size:
            block, within = divmod(offset, self.block_size)
            count = min(size, self.block_size - within)
            physical = self.blocks[block]
            if physical is None:
                output.extend(bytes(count))
            else:
                self.file.seek(physical + within)
                part = self.file.read(count)
                if len(part) != count:
                    raise ValueError("Truncated CISO block")
                output.extend(part)
            offset += count
            size -= count
        return bytes(output)

def sections(dol):
    if len(dol) < 0x100:
        raise ValueError("Truncated DOL")
    result = []
    for i in range(18):
        offset, address, size = word(dol, i * 4), word(dol, 0x48 + i * 4), word(dol, 0x90 + i * 4)
        if size:
            if offset < 0x100 or offset + size > len(dol) or address + size > 0x81800000 or address < 0x80000000:
                raise ValueError("Invalid DOL section bounds")
            result.append((i, offset, address, size))
    return result

def prepare_numeric(dol):
    if hashlib.sha256(dol).hexdigest() != DOL_SHA:
        raise ValueError("Unsupported executable: expected Sonic Adventure 2 Battle USA GSNE8P revision 0")
    mapped = sections(dol)
    data = [s for s in mapped if s[0] >= 7]
    math = bytearray(b"SA2DATA1") + struct.pack(">I", len(data))
    for _, offset, address, size in data:
        math += struct.pack(">II", address, size) + dol[offset:offset + size]
    for _, offset, address, size in mapped:
        target = 0x80196578
        if address <= target and target + 132 <= address + size:
            physics = dol[offset + target - address:offset + target - address + 132]
            return bytes(math), physics
    raise ValueError("Missing Sonic physics table")

def read_inputs(path, animations):
    if path.is_dir():
        dol = (path / "main.dol").read_bytes()
        archives = {n: (path / n).read_bytes() for n in ARCHIVES} if animations else {}
        return dol, archives
    image = Image(path)
    try:
        boot = image.read(0, min(0x440, image.size))
        if len(boot) >= 0x440 and word(boot, 0x1c) == 0xc2339f3d:
            if boot[:6] != b"GSNE8P" or boot[7] != 0:
                raise ValueError("Only GSNE8P revision 0 is currently supported")
            offset = word(boot, 0x420)
            header = image.read(offset, 0x100)
            extent = max([0x100] + [word(header, i * 4) + word(header, 0x90 + i * 4) for i in range(18) if word(header, 0x90 + i * 4)])
            dol = image.read(offset, extent)
            archives = {}
            if animations:
                fst = image.read(word(boot, 0x424), word(boot, 0x428))
                count = word(fst, 8)
                names = count * 12
                if not count or names > len(fst):
                    raise ValueError("Invalid filesystem table")
                for i in range(1, count):
                    entry = word(fst, i * 12)
                    if entry >> 24:
                        continue
                    start = names + (entry & 0xffffff)
                    end = fst.find(b"\0", start)
                    if start >= len(fst) or end < start:
                        raise ValueError("Invalid filename")
                    name = fst[start:end].decode("ascii")
                    if name in ARCHIVES:
                        if name in archives:
                            raise ValueError("Ambiguous archive name: " + name)
                        archives[name] = image.read(word(fst, i * 12 + 4), word(fst, i * 12 + 8))
                if set(archives) != set(ARCHIVES):
                    raise ValueError("The required Sonic animation/model archives were not found")
            return dol, archives
        if animations:
            raise ValueError("Animation import needs an ISO/CISO or a directory containing main.dol and the three PRS archives")
        return image.read(0, image.size), {}
    finally:
        image.close()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", required=True, type=Path, help="ISO, CISO, main.dol, or extracted directory")
    parser.add_argument("--animations", action="store_true", help="Also create the CPU pose bundle from local model/motion archives")
    args = parser.parse_args()
    dol, archives = read_inputs(args.game, args.animations)
    math, physics = prepare_numeric(dol)  # Validate completely before writing.
    output = ROOT / "playable/assets"
    output.mkdir(parents=True, exist_ok=True)
    for name, data in (("original-math-data.bin", math), ("physics.bin", physics)):
        (output / name).write_bytes(data)
    hashes = {"dol_sha256": DOL_SHA, "math_sha256": hashlib.sha256(math).hexdigest(), "physics_sha256": hashlib.sha256(physics).hexdigest()}
    (output / "import-manifest.json").write_text(json.dumps(hashes, indent=2))
    if args.animations:
        extracted = ROOT / ".local/extracted"
        extracted.mkdir(parents=True, exist_ok=True)
        (extracted / "main.dol").write_bytes(dol)
        for name, data in archives.items():
            (extracted / name).write_bytes(data)
        target = ROOT / ".local/animation"
        target.mkdir(parents=True, exist_ok=True)
        subprocess.run(["cargo", "run", "--manifest-path", str(ROOT / "runtime/Cargo.toml"), "--locked", "--bin", "prepare-animation-bundle", "--", str(extracted), str(target / "animation-bundle-audit.json")], check=True)
    print("LOCAL_ASSET_IMPORT_OK")

if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, struct.error, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error))

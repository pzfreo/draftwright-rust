#!/usr/bin/env python3
"""Report transfer sizes without changing the released .wasm file.

Optional: install the Python brotli module to include Brotli quality 11.
"""
import gzip
from pathlib import Path
import sys
import shutil
import subprocess

payload = Path(sys.argv[1]).read_bytes()
sizes = {"raw": len(payload), "gzip -9": len(gzip.compress(payload, compresslevel=9, mtime=0))}
try:
    import brotli
except ImportError:
    if executable := shutil.which("brotli"):
        sizes["Brotli -q 11"] = len(subprocess.run(
            [executable, "-q", "11", "-c"], input=payload, check=True, capture_output=True
        ).stdout)
else:
    sizes["Brotli -q 11"] = len(brotli.compress(payload, quality=11))
for label, count in sizes.items():
    print(f"{label}: {count:,} bytes ({count / 1_000_000:.3f} MB; {count / 1_048_576:.3f} MiB)")

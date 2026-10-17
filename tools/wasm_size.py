#!/usr/bin/env python3
"""Report transfer sizes and enforce the reviewed raw WASM size ceiling."""
import argparse
import gzip
import json
from pathlib import Path
import shutil
import subprocess
import sys

BUDGET = Path(__file__).with_name("wasm-size-budget.json")


def measure(payload):
    sizes = {"raw": len(payload), "gzip": len(gzip.compress(payload, compresslevel=9, mtime=0))}
    try:
        import brotli
    except ImportError:
        if executable := shutil.which("brotli"):
            sizes["brotli"] = len(subprocess.run(
                [executable, "-q", "11", "-w", "22", "-c"], input=payload, check=True, capture_output=True
            ).stdout)
    else:
        sizes["brotli"] = len(brotli.compress(payload, quality=11, lgwin=22))
    return sizes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wasm", type=Path)
    parser.add_argument("--budget", type=Path, default=BUDGET)
    args = parser.parse_args()
    try:
        sizes = measure(args.wasm.read_bytes())
        ceiling = json.loads(args.budget.read_text())["ceiling_bytes"]
        if not isinstance(ceiling, int) or ceiling <= 0:
            raise ValueError("ceiling_bytes must be a positive integer")
        for label, count in sizes.items():
            print(f"{label}: {count:,} bytes ({count / 1_000_000:.3f} MB; {count / 1_048_576:.3f} MiB)")
        headroom = ceiling - sizes["raw"]
        print(f"Raw WASM ceiling: {ceiling:,} bytes; headroom: {headroom:+,} bytes "
              f"({headroom / ceiling:.1%} of ceiling)")
        if headroom < 0:
            print(f"WASM size exceeds budget. Reduce it or deliberately edit {args.budget}.", file=sys.stderr)
            return 1
        return 0
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"WASM size check failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

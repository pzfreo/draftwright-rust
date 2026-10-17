#!/usr/bin/env python3
"""Compare wasm-pack declarations with the reviewed, normalized API snapshot."""
import argparse
import difflib
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = Path("crates/draftwright-wasm/api.d.ts")
GENERATED = Path("crates/draftwright-wasm/pkg/draftwright_wasm.d.ts")
# Preserve string literals while removing generated documentation/version comments.
LEXEMES = re.compile(r'''"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|/\*.*?\*/|//[^\n]*''', re.S)


def statements(source):
    """Split wasm-bindgen's declaration grammar without splitting nested types."""
    depth = 0
    start = 0
    literals = {match.start(): match for match in LEXEMES.finditer(source)}
    i = 0
    while i < len(source):
        if i in literals:
            i = literals[i].end()
            continue
        char = source[i]
        if char in "{([":
            depth += 1
        elif char in "})]":
            depth -= 1
        block_end = char == "}" and re.match(
            r"\s*(?:export\s+)?(?:declare\s+)?(?:class|interface)\b", source[start:]
        )
        if depth == 0 and (char == ";" or block_end):
            yield source[start:i + 1].strip()
            start = i + 1
        i += 1
    if source[start:].strip():
        yield source[start:].strip()


def normalize(source):
    source = LEXEMES.sub(lambda m: " " if m[0].startswith(("/*", "//")) else m[0], source)
    blocks = []
    for declaration in statements(source):
        if re.match(r"(?:export\s+)?(?:declare\s+)?(?:class|interface)\b", declaration):
            header, body = declaration.split("{", 1)
            members = sorted((s.strip() for s in statements(body.rsplit("}", 1)[0])),
                             key=lambda s: s.split("(", 1)[0].split(":", 1)[0])
            declaration = header.strip() + " {\n" + "\n".join(
                "  " + compact(member) for member in members
            ) + "\n}"
        else:
            declaration = compact(declaration)
        blocks.append(declaration)
    return "\n\n".join(sorted(blocks)) + "\n"


def compact(source):
    # Collapse layout whitespace outside literals only.
    parts, start = [], 0
    for match in LEXEMES.finditer(source):
        parts.extend((re.sub(r"\s+", " ", source[start:match.start()]), match[0]))
        start = match.end()
    parts.append(re.sub(r"\s+", " ", source[start:]))
    return "".join(parts).strip()


def wasm_api_notes(changelog):
    unreleased = re.search(r"(?ms)^## \[?Unreleased\]?[^\n]*\n(.*?)(?=^## |\Z)", changelog)
    if not unreleased:
        return set()
    section = re.search(r"(?ms)^### WASM API\s*\n(.*?)(?=^#{1,3} |\Z)", unreleased[1])
    if not section:
        return set()
    return {line.strip() for line in section[1].splitlines() if line.strip()}


def git_file(base, path, missing_ok=False):
    if missing_ok:
        exists = subprocess.run(["git", "cat-file", "-e", f"{base}:{path}"], cwd=ROOT,
                                capture_output=True)
        if exists.returncode:
            return ""
    return subprocess.check_output(["git", "show", f"{base}:{path}"], cwd=ROOT, text=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generated", type=Path, default=ROOT / GENERATED)
    parser.add_argument("--snapshot", type=Path, default=ROOT / SNAPSHOT)
    parser.add_argument("--changelog", type=Path, default=ROOT / "CHANGELOG.md")
    parser.add_argument("--base-ref", default="HEAD", help="PR base or previous main commit (default: HEAD)")
    parser.add_argument("--update", action="store_true", help="write the normalized reviewed snapshot")
    args = parser.parse_args()
    try:
        generated = normalize(args.generated.read_text())
        if args.update:
            args.snapshot.write_text(generated)
            print(f"Updated {args.snapshot}; review the diff and add an Unreleased / WASM API note.")
            return 0
        subprocess.run(["git", "rev-parse", "--verify", f"{args.base_ref}^{{commit}}"],
                       cwd=ROOT, check=True, capture_output=True)
        snapshot = normalize(args.snapshot.read_text())
        previous = git_file(args.base_ref, SNAPSHOT, missing_ok=True)
        changed = generated != snapshot or (previous and generated != normalize(previous))
        if not previous:
            changed = True  # First snapshot is itself a reviewed API addition.
        if not changed:
            print("WASM API matches the reviewed snapshot.")
            return 0
        print("".join(difflib.unified_diff(snapshot.splitlines(True), generated.splitlines(True),
                                           fromfile=str(args.snapshot), tofile=str(args.generated))), end="")
        before = wasm_api_notes(git_file(args.base_ref, "CHANGELOG.md"))
        added = wasm_api_notes(args.changelog.read_text()) - before
        if added:
            print("WASM API change acknowledged by a new Unreleased / WASM API changelog line.")
            if generated != snapshot:
                print("Update the reviewed snapshot: python3 tools/check_wasm_api.py --update")
            return 0
        print("WASM API changed without a new CHANGELOG.md Unreleased / WASM API line.\n"
              "Review and update the snapshot: python3 tools/check_wasm_api.py --update\n"
              "Add a line under ### WASM API in ## Unreleased in the same change.", file=sys.stderr)
        return 1
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        print(f"WASM API check failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())

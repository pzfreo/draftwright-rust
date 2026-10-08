"""Capture draftwright's sheet facts for parts outside the corpus: tools/capture_sheet_facts.py's
``capture()``, with the parts' directory given, written as ``<name>.facts.json.gz``.

    PYTHONPATH=<draftwright>/src python capture.py <capture_sheet_facts.py> <draftwright>/src \\
        <parts dir> <out dir> name ...
"""

import gzip
import json
import sys
from pathlib import Path

tool, src, parts, out, *names = sys.argv[1:]
sys.argv = [tool, src]
sys.path.insert(0, str(Path(tool).resolve().parent))
import capture_sheet_facts  # noqa: E402

for name in names:
    facts = capture_sheet_facts.capture(Path(parts), f"{name}.step")
    text = json.dumps(facts, allow_nan=False, ensure_ascii=False, sort_keys=True, indent=0) + "\n"
    (Path(out) / f"{name}.facts.json.gz").write_bytes(gzip.compress(text.encode(), mtime=0))

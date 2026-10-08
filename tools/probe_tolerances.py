"""Print draftwright's labels for tolerances and fits declared on a part, for tests/tolerances.rs.

    ~/repos/draftwright/.venv/bin/python tools/probe_tolerances.py <draftwright-src> <step> '<ops>'

Seeds a ``Sheet`` from the part's detected features (``Sheet.from_part``), declares each op, builds
the sheet and prints every annotation's name, view, PDF text runs and label (``ANN …``). An op is
``[target, verb, args, kwargs]``: *target* is a feature's index in ``sheet.features`` or
``"envelope"``; *verb* is ``tolerance`` or ``fit``, called on the feature's handle (``of(i)`` for a
hole, boss or step, the parameter handle otherwise) with *args* and *kwargs*. A declaration
draftwright refuses is printed as ``RAISED …``. ``tests/tolerances.rs`` quotes the ops it was run
with beside the labels it compares.
"""

import json
import sys
import warnings

warnings.simplefilter("ignore")
sys.path.insert(0, sys.argv[1])

from build123d import import_step  # noqa: E402

from draftwright.pdf_text import pdf_text_runs  # noqa: E402
from draftwright.sheet import Sheet, _Params  # noqa: E402

sheet = Sheet.from_part(import_step(sys.argv[2]))
for target, verb, args, kwargs in json.loads(sys.argv[3]):
    try:
        if target == "envelope":
            handle = sheet.envelope()
        elif sheet.features[target].kind in ("hole", "boss", "step"):
            handle = sheet.of(target)
        else:
            handle = _Params(sheet, target)
        getattr(handle, verb)(*args, **kwargs)
    except Exception as e:  # noqa: BLE001 - a refusal is what is being recorded
        print("RAISED", target, verb, args, kwargs, type(e).__name__, e)
drawing = sheet.build()
registry = drawing.registry
for name, annotation in drawing.iter_annotations():
    runs = pdf_text_runs(drawing.draft, [(name, annotation)], dimension_spec_of=registry.dimension_spec_of)
    print("ANN", name, "|", registry.view_of(name), "|", [r.text for r in runs], "|", getattr(annotation, "label", None))

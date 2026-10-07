"""Compare the port's PDFs with draftwright's: each page rasterised, and its text extracted.

    ~/repos/draftwright/.venv/bin/python tools/compare_pdf.py target/output

For each captured drawing (``tests/fixtures/output/<name>.pdf``) and the port's
(``<dir>/<name>.pdf``): the page sizes, the share of ink pixels that differ at 200 dpi (and
that are more than one pixel from the other's ink), and the extracted text, run by run.
"""

import sys
from pathlib import Path

import numpy as np
import pypdfium2 as pdfium
from scipy import ndimage

ROOT = Path(__file__).resolve().parent.parent


def page(path):
    doc = pdfium.PdfDocument(str(path))
    p = doc[0]
    ink = np.asarray(p.render(scale=200 / 72, grayscale=True).to_pil()) < 200
    text = p.get_textpage().get_text_range()
    return p.get_size(), ink, sorted(t.strip() for t in text.splitlines() if t.strip())


def main():
    out = Path(sys.argv[1])
    for theirs in sorted((ROOT / "tests" / "fixtures" / "output").glob("*.pdf")):
        (size_a, ink_a, text_a), (size_b, ink_b, text_b) = page(theirs), page(out / theirs.name)
        near_a, near_b = ndimage.binary_dilation(ink_a, iterations=1), ndimage.binary_dilation(ink_b, iterations=1)
        only_a, only_b = ink_a & ~near_b, ink_b & ~near_a
        print(
            f"{theirs.stem}: size {size_a} vs {size_b}; ink {ink_a.sum()} vs {ink_b.sum()} px, "
            f"differing beyond a pixel {only_a.sum()} / {only_b.sum()} "
            f"({only_a.sum() / max(ink_a.sum(), 1):.2e} / {only_b.sum() / max(ink_b.sum(), 1):.2e}); "
            f"text {'same' if text_a == text_b else f'differs: {set(text_a) ^ set(text_b)}'}"
        )


if __name__ == "__main__":
    main()

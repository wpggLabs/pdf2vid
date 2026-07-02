"""Generate the Phase 2.5 PDF fixture set.

Run from repo root:

    python scripts/gen_pdf_fixtures.py

Outputs (deterministic — no timestamps, no UUIDs):

    fixtures/clean-text-3page.pdf       3 pages of clean selectable English text
    fixtures/mixed-blank-page.pdf       4 pages, page 2 is blank (no selectable text)
    fixtures/non-english-3page.pdf      3 pages of selectable Spanish text
    fixtures/scanned-or-image-page.pdf  4 pages, every page is image-only

The fixtures are committed to the repo so tests can run without
invoking this script. This script exists so we can regenerate them
whenever the parser or the test assertions change.

Each fixture is verified immediately after generation by parsing it
back through pypdfium2 and confirming:
  - the expected number of pages
  - which pages have selectable text (text length > 5)
  - text content (first 60 chars) for spot-check
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import Iterable

try:
    from fpdf import FPDF
except ImportError as exc:
    print(f"fpdf2 is required: pip install fpdf2  ({exc})", file=sys.stderr)
    raise

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURES_DIR = REPO_ROOT / "fixtures"

CLEAN_PAGES: list[str] = [
    "Welcome to the sample document. This first page introduces the project. "
    "We will explore three pages of selectable text. Each page becomes one scene in the video. "
    "The narration script is the first three sentences extracted from the page.",
    "Page two covers the architecture. The renderer uses FFmpeg with the drawtext filter for captions. "
    "When the drawtext font is missing the export falls back to text-less video and a typed warning is recorded. "
    "Translation providers can swap source scripts for translated scripts.",
    "The third page wraps up. Each scene has a thumbnail, a duration, and a script. "
    "You can edit any of these in the inspector before exporting. "
    "Export produces a YouTube 1920 by 1080 video and a TikTok 1080 by 1920 video from the same scenes.",
]

MIXED_PAGES: list[str] = [
    "First page of the mixed fixture. The next page will be intentionally blank. "
    "We use this to confirm that the parser records the skipped page and continues with the remaining text. "
    "Skipped pages show up in the import summary and as a typed ProjectWarning.",
    "",  # intentionally blank: pdfium/pypdfium2 will yield no text
    "Third page continues after the blank. The parser should keep page ordering. "
    "Each scene's page number matches the original PDF page number, not the scene index. "
    "This matters for the manual QA outputs.",
    "Fourth page closes the mixed fixture. If you re-import this PDF the same scene set is produced.",
]

SPANISH_PAGES: list[str] = [
    "Bienvenido al documento de muestra. Esta primera pagina introduce el proyecto. "
    "Exploraremos tres paginas con texto seleccionable. Cada pagina se convierte en una escena del video. "
    "El guion de narracion son las primeras tres oraciones extraidas de la pagina.",
    "La segunda pagina cubre la arquitectura. El renderizador usa FFmpeg con el filtro drawtext para los subtitulos. "
    "Cuando falta la fuente drawtext la exportacion recurre a video sin texto y se registra una advertencia tipada. "
    "Los proveedores de traduccion pueden intercambiar el guion original por uno traducido.",
    "La tercera pagina cierra el documento. Cada escena tiene una miniatura, una duracion y un guion. "
    "Puedes editar cualquiera de estos en el inspector antes de exportar. "
    "La exportacion produce un video de YouTube de 1920 por 1080 y un video de TikTok de 1080 por 1920.",
]

# Image-only fixture: every page is a raster image drawn into the PDF,
# so no selectable text exists. The parser must report all 4 pages as
# skipped and fail the import with the OCR error.
IMAGE_ONLY_PAGES: list[bytes] = [b"placeholder"] * 4


def _make_pdf(pages: Iterable[str]) -> bytes:
    """Render an FPDF document from a sequence of text pages.

    Pages whose text is the empty string are added to the PDF but no
    text operators are emitted, so pypdfium2/pdfjs-dist see them as
    having no selectable text. This is what we use to exercise the
    skipped-page code path.
    """
    pdf = FPDF(orientation="P", unit="mm", format="A4")
    pdf.set_auto_page_break(auto=False, margin=10)
    for idx, text in enumerate(pages):
        pdf.add_page()
        if not text:
            # Truly blank: just the page node, no text. Some PDF
            # implementations still write a tiny header; we suppress
            # it by not calling any text/font operator on the page.
            continue
        pdf.set_font("Helvetica", size=18)
        pdf.set_xy(20, 25)
        pdf.cell(0, 10, text=f"Page {idx + 1}", new_x="LMARGIN", new_y="NEXT")
        pdf.set_font("Helvetica", size=12)
        pdf.set_xy(20, 45)
        pdf.multi_cell(170, 7, text=text)
    if hasattr(pdf, "output"):
        try:
            return bytes(pdf.output())
        except TypeError:
            return bytes(pdf.output(dest="S"))  # type: ignore[arg-type]
    raise RuntimeError("FPDF has no output() method")


def _make_image_only_pdf() -> bytes:
    """Render a 4-page PDF whose pages are raster images only.

    Uses fpdf2's image cell to draw a 1x1 PNG (and stretches it) so
    pdfjs-dist sees an image XObject but no text stream. The text
    extraction yields the empty string for every page.
    """
    # 1x1 transparent PNG, base64-decoded once at module load time.
    import base64
    png_bytes = base64.b64decode(
        # 1x1 transparent PNG
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
    )
    pdf = FPDF(orientation="P", unit="mm", format="A4")
    pdf.set_auto_page_break(auto=False, margin=10)
    for idx in range(4):
        pdf.add_page()
        # Place a page number raster image (which fpdf2 embeds as an
        # image XObject). pdfjs-dist will see this as image content
        # only and yield no selectable text.
        pdf.set_xy(20, 20 + idx * 20)
        # We embed the same 1x1 PNG four times; pdfium/pypdfium2 will
        # treat these as image content. Real OCR-required scans would
        # have richer page rasters, but the parser behavior is the
        # same: zero selectable text per page.
        pdf.image(png_bytes, x=20, y=20 + idx * 20, w=8, h=8)
    if hasattr(pdf, "output"):
        try:
            return bytes(pdf.output())
        except TypeError:
            return bytes(pdf.output(dest="S"))  # type: ignore[arg-type]
    raise RuntimeError("FPDF has no output() method")


def _verify(path: Path, expected_pages: int, expected_text_pages: list[int]) -> None:
    """Read the PDF back through pypdfium2 and assert the expected shape."""
    import pypdfium2 as pdfium  # local import keeps the script light

    doc = pdfium.PdfDocument(str(path))
    actual = len(doc)
    if actual != expected_pages:
        raise SystemExit(
            f"{path.name}: expected {expected_pages} pages, got {actual}"
        )
    for idx in range(actual):
        text = doc[idx].get_textpage().get_text_range().strip()
        has_text = len(text) > 5
        should_have_text = idx in expected_text_pages
        if has_text != should_have_text:
            sample = repr(text[:80])
            raise SystemExit(
                f"{path.name} page {idx + 1}: text={has_text} expected={should_have_text} ({sample})"
            )


def main() -> int:
    FIXTURES_DIR.mkdir(parents=True, exist_ok=True)

    fixtures = [
        (
            FIXTURES_DIR / "clean-text-3page.pdf",
            _make_pdf(CLEAN_PAGES),
            3,
            [0, 1, 2],
        ),
        (
            FIXTURES_DIR / "mixed-blank-page.pdf",
            _make_pdf(MIXED_PAGES),
            4,
            # page 2 (index 1) is intentionally blank
            [0, 2, 3],
        ),
        (
            FIXTURES_DIR / "non-english-3page.pdf",
            _make_pdf(SPANISH_PAGES),
            3,
            [0, 1, 2],
        ),
        (
            FIXTURES_DIR / "scanned-or-image-page.pdf",
            _make_image_only_pdf(),
            4,
            # all image-only: no page has selectable text
            [],
        ),
    ]
    for path, data, pages, text_pages in fixtures:
        path.write_bytes(data)
        _verify(path, pages, text_pages)
        print(f"  wrote {path.name} ({len(data)} bytes, {pages} pages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
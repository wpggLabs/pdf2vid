import "@testing-library/jest-dom/vitest";

// pdfjs-dist references `DOMMatrix` at import time. jsdom does not
// implement it, so any test that imports `pdf.ts` (directly or via a
// component) would throw "DOMMatrix is not defined". Provide a minimal
// stub so module load succeeds; the matrix math is not exercised by unit
// tests, only by the real browser render path.
if (typeof (globalThis as { DOMMatrix?: unknown }).DOMMatrix === "undefined") {
  const DOMMatrixStub = function (this: { m11: number; m22: number }) {
    this.m11 = 1;
    this.m22 = 1;
  } as unknown as { new (): { m11: number; m22: number } };
  (globalThis as { DOMMatrix?: unknown }).DOMMatrix = DOMMatrixStub;
}

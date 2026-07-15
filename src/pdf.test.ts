import { describe, expect, it, vi } from "vitest";
import { extractPageText } from "./pdf";

// The heavy PDF import tests live in `src-tauri/tests/pdf_pipeline.rs`,
// which parses the same fixtures with `pdf-extract` and runs them
// through the production render path. Here we unit-test the pure text
// extraction logic from `pdf.ts` with a minimal fake page, so a
// regression in `extractPageText` fails fast without a full PDF engine.

function fakePage(items: Array<{ str?: string }>) {
  return {
    getTextContent: vi.fn(async () => ({ items })),
  } as unknown as import("pdfjs-dist").PDFPageProxy;
}

describe("extractPageText", () => {
  it("joins text items with spaces", async () => {
    const page = fakePage([{ str: "Hello" }, { str: "world" }]);
    const text = await extractPageText(page);
    expect(text).toBe("Hello world");
  });

  it("collapses runs of whitespace into a single space", async () => {
    const page = fakePage([{ str: "a" }, { str: "   b   " }, { str: "c" }]);
    const text = await extractPageText(page);
    expect(text).toBe("a b c");
  });

  it("trims leading/trailing whitespace", async () => {
    const page = fakePage([{ str: "  padded  " }]);
    const text = await extractPageText(page);
    expect(text).toBe("padded");
  });

  it("returns an empty string for image-only pages", async () => {
    const page = fakePage([{ str: "" }, { str: "" }]);
    const text = await extractPageText(page);
    expect(text).toBe("");
  });

  it("ignores non-text items", async () => {
    const page = fakePage([{ str: "keep" }, {}, { str: "this" }]);
    const text = await extractPageText(page);
    expect(text).toBe("keep this");
  });
});

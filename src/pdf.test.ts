import { describe, expect, it } from "vitest";

// The heavy PDF import tests live in `src-tauri/tests/pdf_pipeline.rs`,
// which parses the same fixtures with `pdf-extract` and runs them
// through the production render path. Here we just confirm the
// fixtures are present and well-formed so a CI run can fail fast if
// somebody deletes one.

const FIXTURES_DIR = `${process.cwd()}/fixtures`;

describe("pdf fixtures are present and well-formed", () => {
  it("clean-text-3page.pdf is a valid PDF", async () => {
    const fs = await import("node:fs/promises");
    const bytes = await fs.readFile(`${FIXTURES_DIR}/clean-text-3page.pdf`);
    expect(bytes.length).toBeGreaterThan(1000);
    expect(bytes.subarray(0, 4).toString()).toBe("%PDF");
  });

  it("mixed-blank-page.pdf is a valid PDF", async () => {
    const fs = await import("node:fs/promises");
    const bytes = await fs.readFile(`${FIXTURES_DIR}/mixed-blank-page.pdf`);
    expect(bytes.length).toBeGreaterThan(1000);
    expect(bytes.subarray(0, 4).toString()).toBe("%PDF");
  });

  it("non-english-3page.pdf is a valid PDF", async () => {
    const fs = await import("node:fs/promises");
    const bytes = await fs.readFile(`${FIXTURES_DIR}/non-english-3page.pdf`);
    expect(bytes.length).toBeGreaterThan(1000);
    expect(bytes.subarray(0, 4).toString()).toBe("%PDF");
  });

  it("scanned-or-image-page.pdf is a valid PDF", async () => {
    const fs = await import("node:fs/promises");
    const bytes = await fs.readFile(`${FIXTURES_DIR}/scanned-or-image-page.pdf`);
    expect(bytes.length).toBeGreaterThan(1000);
    expect(bytes.subarray(0, 4).toString()).toBe("%PDF");
  });
});

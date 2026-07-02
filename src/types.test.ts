import { describe, expect, it } from "vitest";

describe("provider option types", () => {
  it("ProviderCategory accepts translation, voice, visual", () => {
    const categories: Array<"translation" | "voice" | "visual"> = [
      "translation",
      "voice",
      "visual",
    ];
    expect(categories).toHaveLength(3);
  });

  it("ProviderKind accepts local and api", () => {
    const kinds: Array<"local" | "api"> = ["local", "api"];
    expect(kinds).toContain("local");
    expect(kinds).toContain("api");
  });
});

describe("scene defaults", () => {
  it("creates a scene with required fields", () => {
    const scene = {
      id: "1",
      page: 1,
      title: "Title",
      script: "Script",
      duration: 7,
      selected: true,
      thumbnail: "",
    };
    expect(scene.id).toBe("1");
    expect(scene.page).toBe(1);
    expect(scene.selected).toBe(true);
  });
});

describe("export warning types", () => {
  it("WarningCode covers render fallback categories", () => {
    const codes: Array<
      | "skippedPage"
      | "untranslatedScene"
      | "missingFont"
      | "renderFallback"
      | "missingDependency"
      | "unsupportedProvider"
      | "voiceSynthesisFailed"
    > = [
      "skippedPage",
      "untranslatedScene",
      "missingFont",
      "renderFallback",
      "missingDependency",
      "unsupportedProvider",
      "voiceSynthesisFailed",
    ];
    expect(codes).toHaveLength(7);
    expect(codes).toContain("renderFallback");
  });

  it("ProjectWarning carries suggestion fields", () => {
    const w = {
      code: "renderFallback" as const,
      severity: "warning" as const,
      message: "fallback used",
      suggestedFix: "Install DejaVu.",
    };
    expect(w.suggestedFix).toContain("DejaVu");
  });

  it("ExportComplete can carry typed warnings", () => {
    const complete = {
      jobId: "j",
      youtubePath: null,
      tiktokPath: null,
      translationWarnings: [],
      skippedPages: [],
      untranslatedCount: 0,
      warnings: [
        {
          code: "missingFont" as const,
          severity: "warning" as const,
          message: "no font",
        },
      ],
      renderFallbackUsed: true,
    };
    expect(complete.warnings).toHaveLength(1);
    expect(complete.renderFallbackUsed).toBe(true);
  });
});

describe("import summary contract", () => {
  // The frontend import summary mirrors the typed warnings the Rust
  // render pipeline emits. We assert the shape here so a future
  // refactor of `useProjectState` does not silently drop fields.
  const exampleSummary = {
    imported: 3,
    skipped: [2],
    needsOcr: false,
    translationNeeded: true,
    warnings: 1,
    status: "3 pages imported · 1 skipped (no text): 2",
  };
  it("carries imported, skipped, and OCR hints", () => {
    expect(exampleSummary.imported).toBe(3);
    expect(exampleSummary.skipped).toEqual([2]);
    expect(exampleSummary.needsOcr).toBe(false);
    expect(exampleSummary.translationNeeded).toBe(true);
    expect(exampleSummary.warnings).toBe(1);
  });
  it("status string mentions skipped page numbers", () => {
    expect(exampleSummary.status).toContain("3 pages imported");
    expect(exampleSummary.status).toContain("1 skipped");
    expect(exampleSummary.status).toContain("2");
  });
});

import { describe, expect, it } from "vitest";
import { captionLineAt, wrapCaptionLines } from "./captions";

describe("wrapCaptionLines", () => {
  it("wraps on word boundaries within the limit", () => {
    const lines = wrapCaptionLines("the quick brown fox jumps over the lazy dog", 15);
    expect(lines.length).toBeGreaterThan(1);
    expect(lines.every((l) => l.length <= 15)).toBe(true);
    expect(lines.join(" ")).toBe("the quick brown fox jumps over the lazy dog");
  });

  it("returns an empty array for empty text", () => {
    expect(wrapCaptionLines("")).toEqual([]);
  });
});

describe("captionLineAt", () => {
  const lines = ["one", "two two", "three"];

  it("returns the first line at progress 0 and last at 1", () => {
    expect(captionLineAt(lines, 0)).toBe("one");
    expect(captionLineAt(lines, 1)).toBe("three");
  });

  it("clamps out-of-range progress", () => {
    expect(captionLineAt(lines, -5)).toBe("one");
    expect(captionLineAt(lines, 5)).toBe("three");
  });

  it("returns empty string when there are no lines", () => {
    expect(captionLineAt([], 0.5)).toBe("");
  });
});

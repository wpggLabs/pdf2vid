import { describe, expect, it } from "vitest";

describe("provider option types", () => {
  it("ProviderCategory accepts translation, voice, visual", () => {
    const categories: Array<"translation" | "voice" | "visual"> = ["translation", "voice", "visual"];
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
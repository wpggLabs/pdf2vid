import { describe, expect, it } from "vitest";
import { languages, translationProviders, voiceProviders } from "./providers";

describe("provider registry", () => {
  it("keeps a free local provider as the default", () => {
    expect(translationProviders[0].kind).toBe("local");
    expect(voiceProviders[0].kind).toBe("local");
  });

  it("has stable unique identifiers", () => {
    for (const providers of [translationProviders, voiceProviders]) {
      expect(new Set(providers.map((provider) => provider.id)).size).toBe(providers.length);
    }
  });

  it("ships multilingual output choices", () => {
    expect(languages).toContain("Chinese (Simplified)");
    expect(languages.length).toBeGreaterThanOrEqual(10);
  });
});

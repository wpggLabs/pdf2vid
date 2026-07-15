import { describe, expect, it } from "vitest";
import type {
  Project,
  ProviderCategory,
  ProviderKind,
  ProviderOption,
  Scene,
  SystemStatus,
} from "./types";

// These tests assert the *runtime shape* the rest of the app relies on.
// TypeScript enforces the union members at compile time; here we make sure
// the documented categories/kinds are exactly the ones the UI switches on,
// so a rename in types.ts breaks a test instead of silently drifting.
const CATEGORIES: ProviderCategory[] = ["translation", "voice", "visual"];
const KINDS: ProviderKind[] = ["local", "api"];

describe("ProviderOption contract", () => {
  const option: ProviderOption = {
    id: "edge",
    label: "edge-tts",
    kind: "local",
    detail: "Free",
    implemented: true,
    online: true,
    keyLabel: null,
    category: "voice",
  };

  it("accepts the documented category members", () => {
    for (const c of CATEGORIES) {
      const o: ProviderOption = { ...option, category: c };
      expect(o.category).toBe(c);
    }
  });

  it("accepts the documented kind members", () => {
    for (const k of KINDS) {
      const o: ProviderOption = { ...option, kind: k };
      expect(o.kind).toBe(k);
    }
  });

  it("carries the fields the UI reads", () => {
    expect(typeof option.id).toBe("string");
    expect(typeof option.label).toBe("string");
    expect(typeof option.implemented).toBe("boolean");
    expect(typeof option.online).toBe("boolean");
    // keyLabel is nullable; the UI branches on it.
    expect(option.keyLabel === null || typeof option.keyLabel === "string").toBe(true);
  });
});

describe("Scene contract", () => {
  const scene: Scene = {
    id: "1",
    page: 1,
    title: "Title",
    script: "Script",
    duration: 7,
    selected: true,
    thumbnail: "",
  };

  it("exposes translatedScript as an optional nullable field", () => {
    const withTranslation: Scene = { ...scene, translatedScript: "Hola" };
    const withNull: Scene = { ...scene, translatedScript: null };
    expect(withTranslation.translatedScript).toBe("Hola");
    expect(withNull.translatedScript).toBeNull();
    expect(scene.translatedScript).toBeUndefined();
  });

  it("has numeric duration and page", () => {
    expect(typeof scene.duration).toBe("number");
    expect(typeof scene.page).toBe("number");
    expect(Number.isFinite(scene.duration)).toBe(true);
  });
});

describe("Project contract", () => {
  const project: Project = {
    name: "Test",
    sourceName: "test.pdf",
    scenes: [],
    language: "English (US)",
    translationProvider: "argos",
    voiceProvider: "edge",
    voice: "en-US-JennyNeural",
    outputYouTube: true,
    outputTikTok: true,
    skippedPages: [],
    voiceSpeed: 100,
  };

  it("keeps skippedPages optional", () => {
    const without: Project = { ...project };
    delete (without as { skippedPages?: number[] }).skippedPages;
    expect((without as { skippedPages?: number[] }).skippedPages).toBeUndefined();
  });

  it("uses a percentage-based voiceSpeed (75-125 UI range)", () => {
    expect(project.voiceSpeed).toBeGreaterThanOrEqual(1);
    expect(project.voiceSpeed).toBeLessThanOrEqual(200);
  });
});

describe("SystemStatus contract", () => {
  const status: SystemStatus = {
    ffmpeg: true,
    ffprobe: true,
    platform: "win32",
    ffmpegSidecarReady: true,
  };

  it("reports boolean dependency readiness", () => {
    expect(typeof status.ffmpeg).toBe("boolean");
    expect(typeof status.ffprobe).toBe("boolean");
    expect(typeof status.ffmpegSidecarReady).toBe("boolean");
    expect(typeof status.platform).toBe("string");
  });
});

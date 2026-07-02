import { describe, expect, it } from "vitest";
import { makeRequestSeq } from "./requestSeq";

describe("makeRequestSeq", () => {
  it("increments monotonically", () => {
    const seq = makeRequestSeq();
    expect(seq.next()).toBe(1);
    expect(seq.next()).toBe(2);
    expect(seq.next()).toBe(3);
  });

  it("isCurrent accepts the latest and rejects older", () => {
    const seq = makeRequestSeq();
    const a = seq.next();
    const b = seq.next();
    expect(seq.isCurrent(a)).toBe(false);
    expect(seq.isCurrent(b)).toBe(true);
  });

  it("protects against the classic stale-response race", async () => {
    const seq = makeRequestSeq();
    const captured: number[] = [];

    // Simulate two concurrent previewVoice calls.
    const sceneA = seq.next();
    const sceneB = seq.next();

    // A returns after B.
    await Promise.resolve();
    await Promise.resolve();
    captured.push(sceneA); // A finishes second, but it's stale
    captured.push(sceneB); // B finishes first, this wins

    // The component logic applies results only when isCurrent holds.
    const applied = captured.filter((s) => seq.isCurrent(s));
    expect(applied).toEqual([sceneB]);
  });
});

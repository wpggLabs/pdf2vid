import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useWorkspaceUi } from "./useWorkspaceUi";

describe("useWorkspaceUi", () => {
  it("starts on the scenes workspace tab", () => {
    const { result } = renderHook(() => useWorkspaceUi());
    expect(result.current.workspaceTab).toBe("scenes");
    expect(result.current.timelineTab).toBe("timeline");
    expect(result.current.inspectorTab).toBe("script");
    expect(result.current.aspect).toBe("youtube");
  });

  it("changes workspace tab", () => {
    const { result } = renderHook(() => useWorkspaceUi());
    act(() => result.current.setWorkspaceTab("preview"));
    expect(result.current.workspaceTab).toBe("preview");
  });

  it("toggles modals independently", () => {
    const { result } = renderHook(() => useWorkspaceUi());
    act(() => result.current.openExport());
    expect(result.current.exportOpen).toBe(true);
    expect(result.current.settingsOpen).toBe(false);
  });

  it("opening models closes settings (single-modal UX)", () => {
    const { result } = renderHook(() => useWorkspaceUi());
    act(() => result.current.setSettingsOpen(true));
    act(() => result.current.openModels());
    expect(result.current.settingsOpen).toBe(false);
    expect(result.current.modelsOpen).toBe(true);
  });
});

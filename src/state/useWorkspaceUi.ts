import { useCallback, useState } from "react";

export type WorkspaceTab = "scenes" | "preview" | "export";
export type TimelineTab = "timeline" | "subtitles";
export type InspectorTab = "script" | "scene";

export interface WorkspaceUi {
  workspaceTab: WorkspaceTab;
  setWorkspaceTab: (t: WorkspaceTab) => void;
  timelineTab: TimelineTab;
  setTimelineTab: (t: TimelineTab) => void;
  inspectorTab: InspectorTab;
  setInspectorTab: (t: InspectorTab) => void;
  aspect: "youtube" | "tiktok";
  setAspect: (a: "youtube" | "tiktok") => void;
  /** Modal open flags */
  settingsOpen: boolean;
  setSettingsOpen: (v: boolean) => void;
  modelsOpen: boolean;
  setModelsOpen: (v: boolean) => void;
  exportOpen: boolean;
  setExportOpen: (v: boolean) => void;
  previewOpen: boolean;
  setPreviewOpen: (v: boolean) => void;
  /** Convenience openers that wire common modal transitions. */
  openSettings: () => void;
  openModels: () => void;
  openExport: () => void;
  /** Tab-action callbacks (kept in the hook so AppShell can stay declarative). */
  handlePreviewTab: () => void;
  toggleFullscreen: () => void;
}

/**
 * Owns all of the UI-only state: tab selection, aspect ratio, which modal
 * is open. AppShell consumes this so its JSX stays focused on layout.
 */
export function useWorkspaceUi(): WorkspaceUi {
  const [workspaceTab, setWorkspaceTab] = useState<WorkspaceTab>("scenes");
  const [timelineTab, setTimelineTab] = useState<TimelineTab>("timeline");
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("script");
  const [aspect, setAspect] = useState<"youtube" | "tiktok">("youtube");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [previewOpen, setPreviewOpen] = useState(false);

  const openSettings = useCallback(() => {
    setModelsOpen(false);
    setSettingsOpen(true);
  }, []);
  const openModels = useCallback(() => {
    setSettingsOpen(false);
    setModelsOpen(true);
  }, []);
  const openExport = useCallback(() => {
    setSettingsOpen(false);
    setExportOpen(true);
  }, []);
  const handlePreviewTab = useCallback(() => setPreviewOpen(true), []);
  const toggleFullscreen = useCallback(() => {
    if (!document.fullscreenElement) {
      document.documentElement.requestFullscreen?.().catch(() => undefined);
    } else {
      document.exitFullscreen?.().catch(() => undefined);
    }
  }, []);

  return {
    workspaceTab,
    setWorkspaceTab,
    timelineTab,
    setTimelineTab,
    inspectorTab,
    setInspectorTab,
    aspect,
    setAspect,
    settingsOpen,
    setSettingsOpen,
    modelsOpen,
    setModelsOpen,
    exportOpen,
    setExportOpen,
    previewOpen,
    setPreviewOpen,
    openSettings,
    openModels,
    openExport,
    handlePreviewTab,
    toggleFullscreen,
  };
}
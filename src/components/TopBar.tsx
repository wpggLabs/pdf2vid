import { Export, Gear, List, Play } from "@phosphor-icons/react";

interface TopBarProps {
  projectName: string;
  workspaceTab: "scenes" | "preview" | "export";
  onScenesTab: () => void;
  onPreviewTab: () => void;
  onExport: () => void;
  onSettings: () => void;
}

export function TopBar({
  projectName,
  workspaceTab,
  onScenesTab,
  onPreviewTab,
  onExport,
  onSettings,
}: TopBarProps) {
  return (
    <header className="topbar">
      <div className="brand">
        <span>pdf2</span>
        <strong>vid</strong>
      </div>
      <div className="project-title">
        <span>Projects</span>
        <b>/</b>
        <strong>{projectName}</strong>
      </div>
      <nav aria-label="Workspace">
        <button
          type="button"
          className={workspaceTab === "scenes" ? "nav-active" : ""}
          onClick={onScenesTab}
        >
          <List size={18} />
          Scenes
        </button>
        <button
          type="button"
          className={workspaceTab === "preview" ? "nav-active" : ""}
          onClick={onPreviewTab}
        >
          <Play size={18} />
          Preview
        </button>
        <button type="button" onClick={onExport}>
          <Export size={18} />
          Export
        </button>
      </nav>
      <button type="button" className="icon-button" aria-label="Settings" onClick={onSettings}>
        <Gear size={20} />
      </button>
    </header>
  );
}

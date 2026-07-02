import { useCallback, useEffect, useRef, useState } from "react";
import { loadProject, saveProject } from "../backend";
import { type PdfImportResult, parsePdf } from "../pdf";
import type { Project, Scene } from "../types";

const demoScenes: Scene[] = [
  {
    id: "welcome",
    page: 1,
    title: "Start with a PDF",
    script: "Import a PDF to build your first narrated video.",
    duration: 7,
    selected: true,
    thumbnail: "",
  },
];

const defaultProject: Project = {
  name: "Untitled project",
  sourceName: "No PDF imported",
  scenes: demoScenes,
  language: "English (US)",
  translationProvider: "argos",
  voiceProvider: "edge",
  voice: "en-US-AriaNeural",
  outputYouTube: true,
  outputTikTok: true,
  voiceSpeed: 100,
};

export interface ImportSummary {
  imported: number;
  skipped: number[];
  /** True when the PDF had no selectable text at all (OCR required). */
  needsOcr: boolean;
  /**
   * True when the imported project still needs the user to pick a
   * translation provider other than the default. We always surface
   * this after import so the UI can show a translation hint.
   */
  translationNeeded: boolean;
  /** Number of warnings associated with the import (== skipped.length). */
  warnings: number;
  /** Flat string for the status bar. */
  status: string;
}

export interface ProjectState {
  project: Project;
  setProject: React.Dispatch<React.SetStateAction<Project>>;
  activeId: string;
  setActiveId: (id: string) => void;
  active: Scene;
  importProgress: { page: number; total: number } | null;
  status: string;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
  /** Structured import summary, refreshed after every PDF load. */
  importSummary: ImportSummary;
  /** Persistently updated from project changes (debounced). */
  importPdf: (file: File) => Promise<void>;
  importPdfFromPath: (path: string) => Promise<void>;
  updateScene: (id: string, changes: Partial<Scene>) => void;
  removeScene: (id: string) => void;
  cancelImport: () => void;
}

/**
 * Owns the canonical Project state plus its derived active scene. Handles
 * hydration from disk, debounced auto-save, and PDF import for both
 * File objects and filesystem paths.
 */
export function useProjectState(): ProjectState {
  const [project, setProject] = useState<Project>(defaultProject);
  const [activeId, setActiveId] = useState(project.scenes[0].id);
  const [importProgress, setImportProgress] = useState<{ page: number; total: number } | null>(
    null,
  );
  const [status, setStatus] = useState("Loading project…");
  const [importSummary, setImportSummary] = useState<ImportSummary>({
    imported: 0,
    skipped: [],
    needsOcr: false,
    translationNeeded: false,
    warnings: 0,
    status: "",
  });
  const saveTimer = useRef<number | null>(null);
  const importAbort = useRef<AbortController | null>(null);

  const active = project.scenes.find((scene) => scene.id === activeId) ?? project.scenes[0];

  // Hydrate from disk on mount
  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        const saved = await loadProject();
        if (!mounted) return;
        if (saved) {
          setProject(saved);
          setActiveId(saved.scenes[0]?.id ?? demoScenes[0].id);
          setStatus("Project loaded");
        } else {
          setStatus("Ready");
        }
      } catch (error) {
        if (mounted) setStatus(`Could not load project: ${error}`);
      }
    })();
    return () => {
      mounted = false;
    };
  }, []);

  // Debounced auto-save
  useEffect(() => {
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveProject(project).catch((error) => setStatus(`Save failed: ${error}`));
    }, 600);
    return () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    };
  }, [project]);

  function formatImportStatus(imported: number, skipped: number[]): string {
    if (skipped.length === 0) {
      return `${imported} pages imported`;
    }
    const sample = skipped.slice(0, 3).join(", ");
    const more = skipped.length > 3 ? `, +${skipped.length - 3} more` : "";
    return `${imported} pages imported · ${skipped.length} skipped (no text): ${sample}${more}`;
  }

  /**
   * Structured import summary surfaced in the inspector right after a
   * PDF loads. We expose both a flat string (for the status bar) and
   * a typed record (for components that want richer UI without
   * parsing the string).
   */
  function buildImportSummary(result: PdfImportResult) {
    const imported = result.scenes.length;
    const skipped = result.skippedPages;
    const needsOcr = imported === 0;
    const translationNeeded =
      imported > 0 &&
      // The default project language is "English (US)" so non-English
      // PDFs will require translation. We can't introspect the actual
      // source language from the import result alone, but we can
      // signal that the user should review the translation panel.
      true;
    return {
      imported,
      skipped,
      needsOcr,
      translationNeeded,
      warnings: skipped.length > 0 ? skipped.length : 0,
      status: formatImportStatus(imported, skipped),
    };
  }

  function applyImportResult(name: string, sourceName: string, result: PdfImportResult) {
    setProject((current) => ({
      ...current,
      name,
      sourceName,
      scenes: result.scenes,
      skippedPages: result.skippedPages,
    }));
    setActiveId(result.scenes[0].id);
    const summary = buildImportSummary(result);
    setImportSummary(summary);
    setStatus(summary.status);
  }

  const importPdf = useCallback(async (file?: File) => {
    if (!file) return;
    importAbort.current = new AbortController();
    setStatus("Reading PDF…");
    setImportProgress({ page: 0, total: 0 });
    try {
      const result = await parsePdf(
        { kind: "file", file },
        (page, total) => {
          setStatus(`Reading page ${page} of ${total}`);
          setImportProgress({ page, total });
        },
        importAbort.current.signal,
      );
      const name = file.name.replace(/\.pdf$/i, "");
      applyImportResult(name, file.name, result);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Could not read this PDF");
    } finally {
      setImportProgress(null);
      importAbort.current = null;
    }
  }, []);

  const importPdfFromPath = useCallback(async (path: string) => {
    importAbort.current = new AbortController();
    setStatus("Reading PDF…");
    setImportProgress({ page: 0, total: 0 });
    try {
      const result = await parsePdf(
        { kind: "path", path },
        (page, total) => {
          setStatus(`Reading page ${page} of ${total}`);
          setImportProgress({ page, total });
        },
        importAbort.current.signal,
      );
      const name =
        path
          .split(/[\\/]/)
          .pop()
          ?.replace(/\.pdf$/i, "") ?? "Untitled";
      applyImportResult(name, path, result);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Could not read this PDF");
    } finally {
      setImportProgress(null);
      importAbort.current = null;
    }
  }, []);

  const updateScene = useCallback((id: string, changes: Partial<Scene>) => {
    setProject((current) => ({
      ...current,
      scenes: current.scenes.map((scene) => (scene.id === id ? { ...scene, ...changes } : scene)),
    }));
  }, []);

  const removeScene = useCallback((id: string) => {
    setProject((current) => {
      if (current.scenes.length === 1) return current;
      const scenes = current.scenes.filter((scene) => scene.id !== id);
      setActiveId(scenes[0].id);
      return { ...current, scenes };
    });
  }, []);

  const cancelImport = useCallback(() => {
    importAbort.current?.abort();
  }, []);

  return {
    project,
    setProject,
    activeId,
    setActiveId,
    active,
    importProgress,
    status,
    setStatus,
    importSummary,
    importPdf,
    importPdfFromPath,
    updateScene,
    removeScene,
    cancelImport,
  };
}

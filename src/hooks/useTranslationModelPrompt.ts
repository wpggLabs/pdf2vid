import { useEffect, useRef, useState } from "react";
import { downloadModel, isModelInstalled } from "../backend";

/**
 * Watches the project for non-English target languages with MarianMT
 * translator. If the model isn't installed, shows a clickable status bar
 * prompt that triggers the download. The hook itself does not block —
 * it's a UI hint, not a gate.
 */
export function useTranslationModelPrompt(
  translationProvider: string,
  language: string,
  onStatus: (message: string, action?: { label: string; run: () => void }) => void,
) {
  const [neededModelId, setNeededModelId] = useState<string | null>(null);
  const [downloading, setDownloading] = useState(false);
  const lastTriggered = useRef<string | null>(null);

  useEffect(() => {
    if (translationProvider !== "marian" || language === "English (US)") {
      setNeededModelId(null);
      return;
    }
    const pair = pairFor(language);
    if (!pair) return;
    const modelId = `marian-${pair}`;
    isModelInstalled(modelId)
      .then((installed) => {
        if (!installed) {
          setNeededModelId(modelId);
          if (lastTriggered.current !== modelId) {
            lastTriggered.current = modelId;
            onStatus(`MarianMT model for ${language} not installed. ~300 MB download required.`, {
              label: "Download",
              run: () => triggerDownload(modelId),
            });
          }
        } else {
          setNeededModelId(null);
          onStatus(`MarianMT model for ${language} ready`);
          lastTriggered.current = modelId;
        }
      })
      .catch(() => undefined);
  }, [translationProvider, language, onStatus]);

  async function triggerDownload(modelId: string) {
    if (downloading) return;
    setDownloading(true);
    onStatus(`Downloading ${modelId}…`);
    try {
      await downloadModel(modelId);
      onStatus(`${modelId} installed`);
      lastTriggered.current = modelId;
      setNeededModelId(null);
    } catch (e) {
      onStatus(`Download failed: ${e}`);
    } finally {
      setDownloading(false);
    }
  }

  return { neededModelId, downloading, triggerDownload };
}

function pairFor(language: string): string | null {
  const map: Record<string, string> = {
    Spanish: "en-es",
    French: "en-fr",
    German: "en-de",
    Portuguese: "en-pt",
    Hindi: "en-hi",
    Japanese: "en-jap",
    Korean: "en-ko",
    "Chinese (Simplified)": "en-zh",
    Arabic: "en-ar",
  };
  return map[language] ?? null;
}

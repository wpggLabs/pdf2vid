import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { Scene } from "./types";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

function sentence(text: string) {
  const cleaned = text.replace(/\s+/g, " ").trim();
  return cleaned.match(/[^.!?。！？]+[.!?。！？]?/g)?.map((item) => item.trim()).filter(Boolean) ?? [];
}

/**
 * Streams a PDF file into pdfjs without loading the whole file into memory.
 * Use the `onProgress` callback for per-page progress, and pass `signal`
 * to cancel a slow parse.
 */
export async function parsePdf(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<Scene[]> {
  // Create a blob URL so pdfjs can stream chunks instead of us
  // materializing the entire ArrayBuffer up front.
  const blobUrl = URL.createObjectURL(file);

  let document;
  try {
    document = await pdfjs.getDocument({
      url: blobUrl,
      // disable range requests if the file:// origin doesn't support them
      disableStream: false,
      disableAutoFetch: false,
    }).promise;
  } catch (e) {
    URL.revokeObjectURL(blobUrl);
    throw e;
  }

  const scenes: Scene[] = [];
  try {
    for (let index = 1; index <= document.numPages; index += 1) {
      if (signal?.aborted) {
        throw new Error("Import cancelled");
      }
      const page = await document.getPage(index);
      const viewport = page.getViewport({ scale: 0.42 });
      const canvas = window.document.createElement("canvas");
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport }).promise;
      const content = await page.getTextContent();
      const text = content.items
        .map((item) => ("str" in item ? item.str : ""))
        .join(" ")
        .replace(/\s+/g, " ")
        .trim();
      if (!text) {
        throw new Error(
          `Page ${index} has no selectable text. Run OCR before importing this PDF.`,
        );
      }
      const script = sentence(text).slice(0, 3).join(" ") || text;
      scenes.push({
        id: crypto.randomUUID(),
        page: index,
        title: script.slice(0, 42),
        script,
        duration: Math.max(4, Math.ceil(script.split(/\s+/).length / 2.5)),
        selected: true,
        thumbnail: canvas.toDataURL("image/jpeg", 0.82),
      });
      onProgress?.(index, document.numPages);
      // Free the page so we don't hold all decoded pages in memory at once.
      page.cleanup();
    }
  } finally {
    URL.revokeObjectURL(blobUrl);
    await document.cleanup();
  }
  return scenes;
}
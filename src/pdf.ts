import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { Scene } from "./types";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

function sentence(text: string) {
  const cleaned = text.replace(/\s+/g, " ").trim();
  return cleaned.match(/[^.!?。！？]+[.!?。！？]?/g)?.map((item) => item.trim()).filter(Boolean) ?? [];
}

/**
 * Parse a PDF file into scenes. Tries blob URL streaming first (less memory
 * for huge PDFs), falls back to ArrayBuffer if the webview blocks the blob
 * fetch (some WebView2 builds do).
 */
export async function parsePdf(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<Scene[]> {
  try {
    return await parsePdfViaBlob(file, onProgress, signal);
  } catch (blobError) {
    console.warn("blob URL parse failed, falling back to ArrayBuffer:", blobError);
    return await parsePdfViaBuffer(file, onProgress, signal);
  }
}

async function parsePdfViaBlob(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<Scene[]> {
  const blobUrl = URL.createObjectURL(file);
  let document;
  try {
    document = await pdfjs.getDocument({
      url: blobUrl,
      disableStream: false,
      disableAutoFetch: false,
    }).promise;
  } catch (e) {
    URL.revokeObjectURL(blobUrl);
    throw e;
  }
  return extractScenes(document, onProgress, signal, blobUrl);
}

async function parsePdfViaBuffer(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<Scene[]> {
  const buffer = await file.arrayBuffer();
  const document = await pdfjs.getDocument({ data: buffer }).promise;
  return extractScenes(document, onProgress, signal, null);
}

async function extractScenes(
  document: Awaited<ReturnType<typeof pdfjs.getDocument>["promise"]>,
  onProgress: ((page: number, total: number) => void) | undefined,
  signal: AbortSignal | undefined,
  blobUrl: string | null,
): Promise<Scene[]> {
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
      page.cleanup();
    }
  } finally {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
    await document.cleanup();
  }
  return scenes;
}
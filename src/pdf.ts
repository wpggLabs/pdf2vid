import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { Scene } from "./types";
import { readPdfFile } from "./backend";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

function sentence(text: string) {
  const cleaned = text.replace(/\s+/g, " ").trim();
  return cleaned.match(/[^.!?。！？]+[.!?。！？]?/g)?.map((item) => item.trim()).filter(Boolean) ?? [];
}

export type PdfSource =
  | { kind: "file"; file: File }
  | { kind: "path"; path: string };

export interface PdfImportResult {
  scenes: Scene[];
  /** Pages that were skipped because they had no selectable text. */
  skippedPages: number[];
}

/**
 * Parse a PDF into scenes. Accepts either a browser File (small/medium PDFs)
 * or a filesystem path (large PDFs read via Rust to avoid JS heap pressure).
 *
 * Tries blob URL streaming first, falls back to ArrayBuffer if the webview
 * blocks the blob fetch.
 *
 * Pages without selectable text are skipped instead of aborting the whole
 * import. If every page lacks text (e.g. scanned PDF), the import fails
 * with a clear error.
 */
export async function parsePdf(
  source: PdfSource,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<PdfImportResult> {
  if (source.kind === "path") {
    return parsePdfViaPath(source.path, onProgress, signal);
  }
  return parsePdfViaFile(source.file, onProgress, signal);
}

async function parsePdfViaPath(
  path: string,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<PdfImportResult> {
  const bytes = await readPdfFile(path);
  const u8 = new Uint8Array(bytes);
  const document = await pdfjs.getDocument({ data: u8 }).promise;
  return extractScenes(document, onProgress, signal);
}

async function parsePdfViaFile(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<PdfImportResult> {
  try {
    return await parsePdfViaBlob(file, onProgress, signal);
  } catch (blobError) {
    console.warn("blob URL parse failed, falling back to ArrayBuffer:", blobError);
    return parsePdfViaBuffer(file, onProgress, signal);
  }
}

async function parsePdfViaBlob(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<PdfImportResult> {
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
  try {
    return await extractScenes(document, onProgress, signal);
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

async function parsePdfViaBuffer(
  file: File,
  onProgress?: (page: number, total: number) => void,
  signal?: AbortSignal,
): Promise<PdfImportResult> {
  const buffer = await file.arrayBuffer();
  const document = await pdfjs.getDocument({ data: new Uint8Array(buffer) }).promise;
  return extractScenes(document, onProgress, signal);
}

async function extractScenes(
  document: Awaited<ReturnType<typeof pdfjs.getDocument>["promise"]>,
  onProgress: ((page: number, total: number) => void) | undefined,
  signal: AbortSignal | undefined,
): Promise<PdfImportResult> {
  const scenes: Scene[] = [];
  const skippedPages: number[] = [];

  for (let index = 1; index <= document.numPages; index += 1) {
    if (signal?.aborted) {
      throw new Error("Import cancelled");
    }
    onProgress?.(index, document.numPages);

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
      // Skip this page; remember it for the status banner.
      skippedPages.push(index);
      page.cleanup();
      continue;
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
    page.cleanup();
  }
  await document.cleanup();

  if (scenes.length === 0) {
    throw new Error(
      skippedPages.length > 0
        ? "No pages had selectable text. Run OCR on this PDF before importing."
        : "PDF had no pages",
    );
  }

  return { scenes, skippedPages };
}
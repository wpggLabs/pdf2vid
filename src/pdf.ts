import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { readPdfFile } from "./backend";
import type { Scene } from "./types";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

function sentence(text: string) {
  const cleaned = text.replace(/\s+/g, " ").trim();
  return (
    cleaned
      .match(/[^.!?。！？]+[.!?。！？]?/g)
      ?.map((item) => item.trim())
      .filter(Boolean) ?? []
  );
}

export type PdfSource = { kind: "file"; file: File } | { kind: "path"; path: string };

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
  let document: Awaited<ReturnType<typeof pdfjs.getDocument>["promise"]>;
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
    const text = await extractPageText(page);

    if (!text) {
      // Skip this page; remember it for the status banner.
      skippedPages.push(index);
      page.cleanup();
      continue;
    }

    const script = sentence(text).slice(0, 3).join(" ") || text;
    const thumbnail = await renderPageThumbnail(page).catch((err) => {
      // Thumbnail rendering is best-effort. We never want it to fail
      // the whole import — the text content is the primary artifact.
      console.warn("thumbnail render failed for page", index, err);
      return "";
    });
    scenes.push({
      id: crypto.randomUUID(),
      page: index,
      title: script.slice(0, 42),
      script,
      duration: Math.max(4, Math.ceil(script.split(/\s+/).length / 2.5)),
      selected: true,
      thumbnail,
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

type PdfPage = import("pdfjs-dist").PDFPageProxy;

/**
 * Extract the trimmed text content of a PDF page. Pulled out of
 * `extractScenes` so it can be unit-tested independently of the
 * canvas rendering path.
 */
export async function extractPageText(page: PdfPage): Promise<string> {
  const content = await page.getTextContent();
  return content.items
    .map((item) => ("str" in item ? item.str : ""))
    .join(" ")
    .replace(/\s+/g, " ")
    .trim();
}

/**
 * Render a high-resolution JPEG for a PDF page. Returns a data URL string
 * suitable for `<img src>` that is *also* the source image for the export.
 *
 * The same image is composited into the 1080p (and 1080×1920 portrait)
 * video, so it must be rendered at a high enough resolution to stay sharp
 * after FFmpeg scales it to fit — plus headroom for the Ken Burns zoom.
 * We target ~2200px on the long edge (roughly 2× the 1080p short edge),
 * which supersamples both the editor preview and the exported frame.
 *
 * Returns an empty string if rendering fails for any reason (the rest of
 * the import must still succeed).
 */
export async function renderPageThumbnail(page: PdfPage): Promise<string> {
  const base = page.getViewport({ scale: 1 });
  const targetLongEdge = 2200;
  // Cap the scale so tiny pages don't blow up the canvas; allow <1 so an
  // already-huge page is downsampled rather than kept at full size.
  const scale = Math.min(4, targetLongEdge / Math.max(base.width, base.height));
  const viewport = page.getViewport({ scale });
  const canvas = window.document.createElement("canvas");
  canvas.width = Math.round(viewport.width);
  canvas.height = Math.round(viewport.height);
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    return "";
  }
  await page.render({ canvas, canvasContext: ctx, viewport }).promise;
  // Near-lossless JPEG keeps text edges crisp while staying far smaller
  // than PNG for a full-colour page.
  return canvas.toDataURL("image/jpeg", 0.95);
}

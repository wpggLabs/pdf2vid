/**
 * Caption helpers shared by the read-along preview. Mirrors the Rust
 * render side (`split_caption_lines` / `timed_caption_lines`) so the
 * editor preview shows the same line-by-line captions as the export.
 */

/** Word-wrap text into short lines (~maxChars each) on word boundaries. */
export function wrapCaptionLines(text: string, maxChars = 42): string[] {
  const lines: string[] = [];
  let current = "";
  for (const word of text.split(/\s+/).filter(Boolean)) {
    if (current && current.length + 1 + word.length > maxChars) {
      lines.push(current);
      current = "";
    }
    current = current ? `${current} ${word}` : word;
  }
  if (current) lines.push(current);
  return lines;
}

/**
 * Pick the caption line for a given playback progress (0..1), weighting
 * each line by its length so timing matches the proportional export.
 * Returns an empty string when there are no lines.
 */
export function captionLineAt(lines: string[], progress: number): string {
  if (lines.length === 0) return "";
  const clamped = Math.max(0, Math.min(1, progress));
  const total = lines.reduce((sum, line) => sum + Math.max(1, line.length), 0);
  const target = clamped * total;
  let acc = 0;
  for (const line of lines) {
    acc += Math.max(1, line.length);
    if (target <= acc) return line;
  }
  return lines[lines.length - 1];
}

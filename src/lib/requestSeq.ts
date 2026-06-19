/**
 * Monotonic request sequence for async operations that should ignore
 * stale responses (e.g. user clicks Skip before the previous scene's
 * previewVoice resolves).
 */
export function makeRequestSeq() {
  let current = 0;
  return {
    next() {
      current += 1;
      return current;
    },
    isCurrent(seq: number) {
      return seq === current;
    },
  };
}
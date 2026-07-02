import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    coverage: {
      provider: "v8",
      reporter: ["text-summary", "html"],
      // Coverage is enforced on the logic layers (hooks, state, lib,
      // pure helpers) that are unit-testable in isolation. The React UI
      // shell (App.tsx, components/*) is verified via manual QA and is
      // intentionally out of the enforced scope until it is refactored
      // into testable pieces.
      include: [
        "src/lib/**/*.ts",
        "src/hooks/**/*.ts",
        "src/state/**/*.ts",
        "src/pdf.ts",
      ],
      exclude: ["src/**/*.test.{ts,tsx}"],
      // Floor set just below the current baseline so regressions fail CI
      // while leaving headroom. Raise these as coverage improves.
      thresholds: {
        statements: 25,
        branches: 10,
        functions: 28,
        lines: 25,
      },
    },
  },
});

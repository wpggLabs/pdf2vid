#!/usr/bin/env node
// Downloads platform-specific FFmpeg static builds and places them
// in src-tauri/binaries/ with Tauri's expected naming convention.
//
// Usage: node scripts/fetch-ffmpeg.js [platform]
//   platform: win-x86_64 | mac-arm64 | linux-x86_64 | all | host
//
// Supply-chain hardening
// ----------------------
// The upstream archives are not pinned by the provider (several use a
// rolling "latest" URL), so we cannot bake a single fixed hash into
// this script without it going stale. Instead we verify against a
// checksum supplied out-of-band, in priority order:
//   1. env var  FFMPEG_SHA256_<KEY>   (KEY upper-cased, `-` -> `_`)
//   2. scripts/ffmpeg-checksums.json  ({ "<key>": "<sha256>" })
// When a checksum is present the download must match or the build
// aborts. When none is present we print a loud warning and record the
// hash we saw, so a maintainer can pin it for release builds.

import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, readFileSync, renameSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { arch, platform } from "node:process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");
const BIN_DIR = join(ROOT, "src-tauri", "binaries");
const CHECKSUMS_FILE = join(__dirname, "ffmpeg-checksums.json");

if (!existsSync(BIN_DIR)) mkdirSync(BIN_DIR, { recursive: true });

const SOURCES = {
  "win-x86_64": {
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
    extract: (zipPath, outDir) => {
      execSync(
        `powershell -NoProfile -Command "Expand-Archive -LiteralPath '${zipPath}' -DestinationPath '${outDir}\\extracted' -Force"`,
        { stdio: "inherit" },
      );
      return execSync(
        `powershell -NoProfile -Command "Get-ChildItem -Recurse '${outDir}\\extracted' -Filter ffmpeg.exe | Select-Object -First 1 -ExpandProperty FullName"`,
        { encoding: "utf8" },
      ).trim();
    },
  },
  "mac-arm64": {
    url: "https://osxexperts.net/ffmpeg7arm.zip",
    extract: (zipPath, outDir) => {
      execSync(`unzip -o "${zipPath}" -d "${outDir}"`, { stdio: "inherit" });
      return join(outDir, "ffmpeg");
    },
  },
  "linux-x86_64": {
    // BtbN GitHub builds are used instead of johnvansickle.com, which
    // returns HTTP 415 to CI/curl requests (hotlink protection).
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz",
    extract: (tarPath, outDir) => {
      execSync(`tar -xJf "${tarPath}" -C "${outDir}"`, { stdio: "inherit" });
      return execSync(`find "${outDir}" -type f -name ffmpeg | head -1`, {
        encoding: "utf8",
      }).trim();
    },
  },
};

const TRIPLES = {
  "win-x86_64": "x86_64-pc-windows-msvc",
  "mac-x86_64": "x86_64-apple-darwin",
  "mac-arm64": "aarch64-apple-darwin",
  "linux-x86_64": "x86_64-unknown-linux-gnu",
};

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function expectedChecksum(key) {
  const envKey = `FFMPEG_SHA256_${key.toUpperCase().replace(/-/g, "_")}`;
  if (process.env[envKey]) return process.env[envKey].trim().toLowerCase();
  if (existsSync(CHECKSUMS_FILE)) {
    try {
      const map = JSON.parse(readFileSync(CHECKSUMS_FILE, "utf8"));
      if (map[key]) return String(map[key]).trim().toLowerCase();
    } catch (e) {
      console.warn(`! Could not parse ${CHECKSUMS_FILE}: ${e.message}`);
    }
  }
  return null;
}

function verify(key, archivePath) {
  const actual = sha256(archivePath);
  const expected = expectedChecksum(key);
  if (!expected) {
    const envKey = `FFMPEG_SHA256_${key.toUpperCase().replace(/-/g, "_")}`;
    console.warn(
      `! No pinned SHA-256 for '${key}'. Downloaded archive hashes to:\n    ${actual}\n` +
        `  Pin it via scripts/ffmpeg-checksums.json or ${envKey} to harden release builds.`,
    );
    return;
  }
  if (actual !== expected) {
    throw new Error(
      `SHA-256 mismatch for '${key}'.\n  expected: ${expected}\n  actual:   ${actual}\n` +
        "  Refusing to use an unverified FFmpeg binary.",
    );
  }
  console.log(`✓ Verified SHA-256 for ${key}`);
}

function download(url, dest) {
  if (existsSync(dest) && statSync(dest).size > 1024 * 1024) {
    console.log(`✓ ${dest} already present (${(statSync(dest).size / 1024 / 1024).toFixed(1)} MB)`);
    return;
  }
  console.log(`↓ Downloading ${url}`);
  // A browser User-Agent avoids 4xx from mirrors that block bare curl.
  const ua =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
  execSync(`curl -L --fail -A "${ua}" -o "${dest}" "${url}"`, {
    stdio: "inherit",
    shell: true,
  });
}

function fetchTarget(key) {
  const source = SOURCES[key];
  if (!source) throw new Error(`Unknown target: ${key}`);
  const workDir = join(BIN_DIR, ".work", key);
  if (!existsSync(workDir)) mkdirSync(workDir, { recursive: true });

  const ext = key.startsWith("win") ? ".zip" : key.startsWith("linux") ? ".tar.xz" : ".zip";
  const archive = join(workDir, `ffmpeg${ext}`);

  download(source.url, archive);
  verify(key, archive);

  const extracted = source.extract(archive, workDir);
  const sidecarName = `ffmpeg-${TRIPLES[key]}${key.startsWith("win") ? ".exe" : ""}`;
  const finalPath = join(BIN_DIR, sidecarName);
  renameSync(extracted, finalPath);
  if (!key.startsWith("win")) chmodSync(finalPath, 0o755);
  console.log(`✓ Sidecar: ${finalPath}`);
  return finalPath;
}

function currentHostKey() {
  if (platform === "win32") return "win-x86_64";
  // CI only ships Apple Silicon macOS builds, so any macOS host targets
  // the arm64 sidecar. (Intel macOS FFmpeg is no longer fetched — see
  // SOURCES and the release workflow matrix.)
  if (platform === "darwin") return "mac-arm64";
  if (platform === "linux") return "linux-x86_64";
  throw new Error(`Unsupported host: ${platform}/${arch}`);
}

function main() {
  const arg = process.argv[2] ?? "host";
  let keys = [];
  if (arg === "host") keys = [currentHostKey()];
  else if (arg === "all") keys = Object.keys(SOURCES);
  else keys = [arg];

  console.log(`Fetching FFmpeg for: ${keys.join(", ")}`);
  for (const key of keys) {
    try {
      fetchTarget(key);
    } catch (e) {
      console.error(`✗ ${key}: ${e.message}`);
      process.exitCode = 1;
    }
  }
}

main();

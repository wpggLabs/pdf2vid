#!/usr/bin/env node
// Downloads platform-specific FFmpeg static builds and places them
// in src-tauri/binaries/ with Tauri's expected naming convention.
//
// Usage: node scripts/fetch-ffmpeg.js [platform]
//   platform: win | mac | linux | all (default: current host)

import { execSync } from "node:child_process";
import { existsSync, mkdirSync, statSync, createWriteStream, renameSync, chmodSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { platform, arch } from "node:process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");
const BIN_DIR = join(ROOT, "src-tauri", "binaries");

if (!existsSync(BIN_DIR)) mkdirSync(BIN_DIR, { recursive: true });

// Tauri sidecar naming convention: <bin>-<target-triple><.exe on Windows>
// See: https://v2.tauri.app/develop/sidecar/
function tauriTarget() {
  const os = platform;
  const a = arch;
  if (os === "win32") return "x86_64-pc-windows-msvc";
  if (os === "darwin") return a === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  return "x86_64-unknown-linux-gnu";
}

const SOURCES = {
  "win-x86_64": {
    url: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
    extract: (zipPath, outDir) => {
      // zip extraction handled by caller via PowerShell Expand-Archive
      const out = execSync(
        `powershell -NoProfile -Command "Expand-Archive -LiteralPath '${zipPath}' -DestinationPath '${outDir}\\extracted' -Force"`,
        { stdio: "inherit" },
      );
      const glob = require("node:child_process")
        .execSync(
          `powershell -NoProfile -Command "Get-ChildItem -Recurse '${outDir}\\extracted' -Filter ffmpeg.exe | Select-Object -First 1 -ExpandProperty FullName"`,
          { encoding: "utf8" },
        )
        .trim();
      return glob;
    },
  },
  "mac-x86_64": {
    url: "https://evermeet.cx/ffmpeg/getrelease/zip",
    extract: (zipPath, outDir) => {
      execSync(`unzip -o "${zipPath}" -d "${outDir}"`, { stdio: "inherit" });
      return join(outDir, "ffmpeg");
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
    url: "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz",
    extract: (tarPath, outDir) => {
      execSync(`tar -xJf "${tarPath}" -C "${outDir}"`, { stdio: "inherit" });
      const glob = execSync(`find "${outDir}" -type f -name ffmpeg | head -1`, {
        encoding: "utf8",
      }).trim();
      return glob;
    },
  },
};

async function download(url, dest) {
  if (existsSync(dest) && statSync(dest).size > 1024 * 1024) {
    console.log(`✓ ${dest} already present (${(statSync(dest).size / 1024 / 1024).toFixed(1)} MB)`);
    return;
  }
  console.log(`↓ Downloading ${url}`);
  execSync(
    `curl -L --fail -o "${dest}" "${url}"`,
    { stdio: "inherit", shell: true },
  );
}

function fetch(targetKey) {
  const source = SOURCES[targetKey];
  if (!source) throw new Error(`Unknown target: ${targetKey}`);
  const workDir = join(BIN_DIR, ".work", targetKey);
  if (!existsSync(workDir)) mkdirSync(workDir, { recursive: true });

  const ext = targetKey.startsWith("win") ? ".zip" : targetKey.startsWith("linux") ? ".tar.xz" : ".zip";
  const archive = join(workDir, `ffmpeg${ext}`);

  return download(source.url, archive).then(() => {
    const extracted = source.extract(archive, workDir);
    const triple = tauriTargetForKey(targetKey);
    const sidecarName = `ffmpeg-${triple}${targetKey.startsWith("win") ? ".exe" : ""}`;
    const finalPath = join(BIN_DIR, sidecarName);
    renameSync(extracted, finalPath);
    if (!targetKey.startsWith("win")) chmodSync(finalPath, 0o755);
    console.log(`✓ Sidecar: ${finalPath}`);
    return finalPath;
  });
}

function tauriTargetForKey(key) {
  if (key === "win-x86_64") return "x86_64-pc-windows-msvc";
  if (key === "mac-x86_64") return "x86_64-apple-darwin";
  if (key === "mac-arm64") return "aarch64-apple-darwin";
  if (key === "linux-x86_64") return "x86_64-unknown-linux-gnu";
  throw new Error(`Unknown key ${key}`);
}

function currentHostKey() {
  const os = platform;
  const a = arch;
  if (os === "win32") return "win-x86_64";
  if (os === "darwin") return a === "arm64" ? "mac-arm64" : "mac-x86_64";
  if (os === "linux") return "linux-x86_64";
  throw new Error(`Unsupported host: ${os}/${a}`);
}

async function main() {
  const arg = process.argv[2] ?? "host";
  let keys = [];
  if (arg === "host") keys = [currentHostKey()];
  else if (arg === "all") keys = Object.keys(SOURCES);
  else keys = [arg];

  console.log(`Fetching FFmpeg for: ${keys.join(", ")}`);
  for (const key of keys) {
    try {
      await fetch(key);
    } catch (e) {
      console.error(`✗ ${key}: ${e.message}`);
      process.exitCode = 1;
    }
  }
}

main();
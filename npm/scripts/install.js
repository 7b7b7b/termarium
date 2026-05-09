#!/usr/bin/env node

const childProcess = require("node:child_process");
const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const packageRoot = path.resolve(__dirname, "..", "..");
const packageJson = require(path.join(packageRoot, "package.json"));
const repo = "https://github.com/7b7b7b/termarium";
const version = packageJson.version;
const downloadRetries = readPositiveIntegerEnv("TERMARIUM_NPM_DOWNLOAD_RETRIES", 3);
const downloadStallTimeoutMs = readPositiveIntegerEnv("TERMARIUM_NPM_DOWNLOAD_STALL_TIMEOUT_MS", 60_000);

const targets = {
  "darwin-arm64": {
    target: "aarch64-apple-darwin",
    archive: "tar.gz",
    binary: "termarium",
  },
  "linux-x64": {
    target: "x86_64-unknown-linux-gnu",
    archive: "tar.gz",
    binary: "termarium",
  },
  "win32-x64": {
    target: "x86_64-pc-windows-msvc",
    archive: "zip",
    binary: "termarium.exe",
  },
};

main().catch((error) => {
  console.error(`termarium postinstall failed: ${error.message}`);
  process.exit(1);
});

async function main() {
  if (process.env.TERMARIUM_NPM_SKIP_DOWNLOAD === "1") {
    console.log("termarium: skipped binary download");
    return;
  }

  const platformKey = `${process.platform}-${process.arch}`;
  const selected = targets[platformKey];
  if (!selected) {
    throw new Error(
      `unsupported platform ${platformKey}. Supported platforms: ${Object.keys(targets).join(", ")}`
    );
  }

  const asset = `termarium-${selected.target}.${selected.archive}`;
  const releaseBase = `${repo}/releases/download/v${version}`;
  const archiveUrl = `${releaseBase}/${asset}`;
  const checksumUrl = `${releaseBase}/SHA256SUMS`;
  const vendorDir = path.join(packageRoot, "npm", "vendor");
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "termarium-npm-"));
  const archivePath = path.join(tmpDir, asset);
  const extractDir = path.join(tmpDir, "extract");

  fs.mkdirSync(vendorDir, { recursive: true });
  fs.mkdirSync(extractDir, { recursive: true });

  try {
    console.log(`termarium: downloading ${asset}`);
    const archive = await download(archiveUrl, { label: asset, progress: true });
    fs.writeFileSync(archivePath, archive);

    if (process.env.TERMARIUM_NPM_SKIP_CHECKSUM !== "1") {
      const checksums = (await download(checksumUrl, { label: "SHA256SUMS" })).toString("utf8");
      verifyChecksum(asset, archive, checksums);
    }

    extractArchive(archivePath, extractDir, selected.archive);

    const extractedBinary = path.join(extractDir, selected.binary);
    if (!fs.existsSync(extractedBinary)) {
      throw new Error(`${selected.binary} was not found in ${asset}`);
    }

    const installedBinary = path.join(vendorDir, selected.binary);
    fs.copyFileSync(extractedBinary, installedBinary);
    if (process.platform !== "win32") {
      fs.chmodSync(installedBinary, 0o755);
    }
    console.log(`termarium: installed ${selected.target} binary`);
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

function verifyChecksum(asset, archive, checksums) {
  const line = checksums
    .split(/\r?\n/)
    .find((candidate) => candidate.includes(asset));
  if (!line) {
    throw new Error(`checksum for ${asset} was not found`);
  }
  const expected = line.trim().split(/\s+/)[0].toLowerCase();
  const actual = crypto.createHash("sha256").update(archive).digest("hex");
  if (actual !== expected) {
    throw new Error(`checksum mismatch for ${asset}`);
  }
}

function extractArchive(archivePath, extractDir, archive) {
  if (archive === "tar.gz") {
    run("tar", ["-xzf", archivePath, "-C", extractDir]);
    return;
  }
  if (archive === "zip") {
    const command = [
      "$ErrorActionPreference = 'Stop'",
      `Expand-Archive -LiteralPath '${escapePowerShell(archivePath)}' -DestinationPath '${escapePowerShell(extractDir)}' -Force`,
    ].join("; ");
    run("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]);
    return;
  }
  throw new Error(`unsupported archive type ${archive}`);
}

function run(command, args) {
  const result = childProcess.spawnSync(command, args, { stdio: "inherit" });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}`);
  }
}

async function download(url, options = {}) {
  let lastError;
  for (let attempt = 1; attempt <= downloadRetries; attempt += 1) {
    try {
      return await downloadOnce(url, options);
    } catch (error) {
      lastError = error;
      if (attempt >= downloadRetries) {
        break;
      }
      console.error(
        `termarium: download failed (${error.message}); retrying ${attempt}/${downloadRetries - 1}`
      );
    }
  }
  throw lastError;
}

function downloadOnce(url, options = {}, redirects = 0) {
  return new Promise((resolve, reject) => {
    const request = https.get(
      url,
      {
        headers: {
          "User-Agent": "termarium-npm-installer",
        },
      },
      (response) => {
        const status = response.statusCode ?? 0;
        if ([301, 302, 303, 307, 308].includes(status)) {
          response.resume();
          if (!response.headers.location) {
            reject(new Error(`redirect from ${url} did not include a location`));
            return;
          }
          if (redirects >= 5) {
            reject(new Error(`too many redirects while downloading ${url}`));
            return;
          }
          resolve(
            downloadOnce(new URL(response.headers.location, url).toString(), options, redirects + 1)
          );
          return;
        }

        if (status < 200 || status >= 300) {
          response.resume();
          reject(new Error(`download failed for ${url}: HTTP ${status}`));
          return;
        }

        const chunks = [];
        const progress = createDownloadProgress(options, response.headers["content-length"]);
        response.on("data", (chunk) => {
          chunks.push(chunk);
          progress.update(chunk.length);
        });
        response.on("end", () => {
          progress.finish();
          resolve(Buffer.concat(chunks));
        });
      }
    );
    request.setTimeout(downloadStallTimeoutMs, () => {
      request.destroy(new Error(`download stalled for more than ${downloadStallTimeoutMs / 1000}s`));
    });
    request.on("error", reject);
  });
}

function createDownloadProgress(options, contentLength) {
  if (!options.progress) {
    return { update() {}, finish() {} };
  }

  const label = options.label ?? "download";
  const total = Number(contentLength);
  const hasTotal = Number.isFinite(total) && total > 0;
  const startedAt = Date.now();
  const useSingleLine = process.stderr.isTTY;
  const intervalMs = useSingleLine ? 100 : 5_000;
  let received = 0;
  let lastRenderAt = 0;
  let finished = false;

  const render = (force = false) => {
    if (finished) {
      return;
    }
    const now = Date.now();
    if (!force && now - lastRenderAt < intervalMs) {
      return;
    }
    lastRenderAt = now;

    const elapsedSeconds = Math.max((now - startedAt) / 1000, 0.001);
    const speed = received / elapsedSeconds;
    const receivedText = formatBytes(received);
    let message;

    if (hasTotal) {
      const percent = Math.min(received / total, 1);
      const eta = speed > 0 ? (total - received) / speed : 0;
      message = [
        `termarium: ${label}`,
        formatPercent(percent),
        progressBar(percent),
        `${receivedText}/${formatBytes(total)}`,
        `${formatBytes(speed)}/s`,
        `ETA ${formatDuration(eta)}`,
      ].join(" ");
    } else {
      message = `termarium: ${label} ${receivedText} ${formatBytes(speed)}/s`;
    }

    if (useSingleLine) {
      process.stderr.write(`\r${message}`);
    } else {
      console.error(message);
    }
  };

  return {
    update(chunkLength) {
      received += chunkLength;
      render(false);
    },
    finish() {
      render(true);
      finished = true;
      if (useSingleLine) {
        process.stderr.write("\n");
      }
    },
  };
}

function progressBar(percent) {
  const width = 20;
  const filled = Math.max(0, Math.min(width, Math.round(percent * width)));
  return `[${"#".repeat(filled)}${"-".repeat(width - filled)}]`;
}

function formatPercent(percent) {
  return `${Math.round(percent * 100).toString().padStart(3, " ")}%`;
}

function formatBytes(bytes) {
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const precision = value >= 100 || unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
}

function formatDuration(seconds) {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return "--";
  }
  if (seconds < 60) {
    return `${Math.ceil(seconds)}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.ceil(seconds % 60);
  return `${minutes}m${remainingSeconds.toString().padStart(2, "0")}s`;
}

function readPositiveIntegerEnv(name, fallback) {
  const value = Number(process.env[name]);
  if (Number.isInteger(value) && value > 0) {
    return value;
  }
  return fallback;
}

function escapePowerShell(value) {
  return value.replace(/'/g, "''");
}

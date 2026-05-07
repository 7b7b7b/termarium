#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const executable = process.platform === "win32" ? "termarium.exe" : "termarium";
const bundledBinary =
  process.env.TERMARIUM_BINARY_PATH ||
  path.join(__dirname, "..", "vendor", executable);

if (!fs.existsSync(bundledBinary)) {
  console.error("Termarium binary is missing.");
  console.error("Try reinstalling with: npm install -g termarium --force");
  process.exit(1);
}

const result = spawnSync(bundledBinary, process.argv.slice(2), {
  stdio: "inherit",
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 0);

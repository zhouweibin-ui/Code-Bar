import assert from "node:assert/strict";
import { test } from "node:test";

import { buildTerminalLaunchConfig } from "../src/components/terminalLaunchConfig.ts";

test("windows terminal widgets launch PowerShell 7 by default", () => {
  const config = buildTerminalLaunchConfig("D:\\code\\OpenSource\\Code-Bar", true);

  assert.equal(config.command, "pwsh.exe");
  assert.deepEqual(config.args, [
    "-NoLogo",
    "-NoExit",
    "-Command",
    "Set-Location -LiteralPath 'D:\\code\\OpenSource\\Code-Bar'",
  ]);
});

test("powershell workdir quoting escapes single quotes", () => {
  const config = buildTerminalLaunchConfig("D:\\work\\Bob's Repo", true);

  assert.equal(config.args[3], "Set-Location -LiteralPath 'D:\\work\\Bob''s Repo'");
});

test("non-windows terminal widgets keep the existing zsh launch behavior", () => {
  const config = buildTerminalLaunchConfig("/mnt/d/code/OpenSource/Code-Bar", false);

  assert.equal(config.command, "sh");
  assert.deepEqual(config.args, ["-lc", "cd '/mnt/d/code/OpenSource/Code-Bar' && exec zsh -i"]);
});

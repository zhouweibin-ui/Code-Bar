import assert from "node:assert/strict";
import { test } from "node:test";

import { shouldAutoOpenRunnerSurface } from "../src/hooks/sessionRunnerState.ts";

test("resume-bound idle Codex sessions auto-open the runner surface", () => {
  assert.equal(
    shouldAutoOpenRunnerSurface({
      status: "idle",
      runner: { type: "codex" },
      providerSessionId: "019de232-e31a-7a72-94a7-e25269572067",
    }),
    true,
  );
});

test("idle sessions without provider bindings keep the prompt composer visible", () => {
  assert.equal(
    shouldAutoOpenRunnerSurface({
      status: "idle",
      runner: { type: "codex" },
      providerSessionId: "",
    }),
    false,
  );
});

test("running sessions keep the runner surface visible without provider bindings", () => {
  assert.equal(
    shouldAutoOpenRunnerSurface({
      status: "running",
      runner: { type: "codex" },
    }),
    true,
  );
});

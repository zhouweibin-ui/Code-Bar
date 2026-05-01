import assert from "node:assert/strict";
import { test } from "node:test";

import { nextMountedSessionPanelIds } from "../src/hooks/sessionPanelMountState.ts";

test("keeps the visible session mounted and prunes idle history panels", () => {
  const next = nextMountedSessionPanelIds(
    ["10", "11"],
    "11",
    [
      { id: "10", status: "idle" },
      { id: "11", status: "idle" },
    ],
  );

  assert.deepEqual(next, ["11"]);
});

test("prunes running background panels so resume PTYs do not accumulate", () => {
  const next = nextMountedSessionPanelIds(
    ["10", "11"],
    "11",
    [
      { id: "10", status: "running" },
      { id: "11", status: "idle" },
    ],
  );

  assert.deepEqual(next, ["11"]);
});

test("adds a newly visible session after pruning stale panels", () => {
  const next = nextMountedSessionPanelIds(
    ["10"],
    "12",
    [
      { id: "10", status: "done" },
      { id: "12", status: "idle" },
    ],
  );

  assert.deepEqual(next, ["12"]);
});

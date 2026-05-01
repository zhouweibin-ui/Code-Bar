import assert from "node:assert/strict";
import { test } from "node:test";

import {
  buildSessionDeleteDialogState,
  getSessionDeleteDialogMode,
  getSessionWorkspacePath,
} from "../src/components/sessionDeleteLogic.ts";

const session = { id: "1", workspaceId: "workspace-a" };

test("uses the session workspace path instead of the active workspace", () => {
  const path = getSessionWorkspacePath(session, [
    { id: "workspace-b", path: "/repo/b" },
    { id: "workspace-a", path: "/repo/a" },
  ]);

  assert.equal(path, "/repo/a");
});

test("safe inspections still produce a confirmation dialog state", () => {
  const state = buildSessionDeleteDialogState(session, {
    safety: {
      stagedCount: 0,
      unstagedCount: 0,
      untrackedCount: 0,
      conflictCount: 0,
      aheadCount: 0,
      hasUncommittedChanges: false,
      hasUnmergedCommits: false,
    },
  });

  assert.equal(state.session, session);
  assert.equal(getSessionDeleteDialogMode(state), "safe");
});

test("inspection failures produce an error confirmation dialog state", () => {
  const state = buildSessionDeleteDialogState(session, { error: "missing base branch" });

  assert.equal(state.session, session);
  assert.equal(getSessionDeleteDialogMode(state), "error");
});

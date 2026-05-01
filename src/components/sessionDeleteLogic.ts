export type SessionDeleteSafety = {
  stagedCount: number;
  unstagedCount: number;
  untrackedCount: number;
  conflictCount: number;
  aheadCount: number;
  hasUncommittedChanges: boolean;
  hasUnmergedCommits: boolean;
};

export type SessionDeleteDialogMode = "safe" | "risk" | "error";

export type SessionDeleteDialogState<TSession> = {
  session: TSession;
  safety?: SessionDeleteSafety;
  error?: string;
};

export type DeleteDialogViewport = {
  width: number;
  height: number;
};

export function hasSessionDeleteRisk(safety: SessionDeleteSafety) {
  return safety.hasUncommittedChanges || safety.hasUnmergedCommits;
}

export function getUncommittedChangeCount(safety: SessionDeleteSafety) {
  return safety.stagedCount + safety.unstagedCount + safety.untrackedCount + safety.conflictCount;
}

export function getSessionWorkspacePath<TSession extends { workspaceId: string }, TWorkspace extends { id: string; path: string }>(
  session: TSession,
  workspaces: TWorkspace[]
) {
  // 删除确认弹窗可能在用户切换 workspace 后才确认，必须使用 session 归属的 workspace。
  return workspaces.find((workspace) => workspace.id === session.workspaceId)?.path;
}

export function buildSessionDeleteDialogState<TSession>(
  session: TSession,
  result: { safety?: SessionDeleteSafety; error?: string } = {}
): SessionDeleteDialogState<TSession> {
  return {
    session,
    ...(result.safety ? { safety: result.safety } : {}),
    ...(result.error ? { error: result.error } : {}),
  };
}

export function getSessionDeleteDialogMode(state: SessionDeleteDialogState<unknown>): SessionDeleteDialogMode {
  if (state.error) return "error";
  if (state.safety && hasSessionDeleteRisk(state.safety)) return "risk";
  return "safe";
}

export function getDeleteDialogViewportLayout(viewport: DeleteDialogViewport) {
  return {
    compact: viewport.width < 360 || viewport.height < 260,
    wrapActions: viewport.width < 320,
  };
}

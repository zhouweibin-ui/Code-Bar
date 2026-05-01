type PanelSessionStatus = "idle" | "running" | "waiting" | "suspended" | "done" | "error";

interface PanelSession {
  id: string;
  status: PanelSessionStatus;
}

export function nextMountedSessionPanelIds(
  previousIds: string[],
  visibleSessionId: string | null | undefined,
  sessions: PanelSession[],
): string[] {
  const byId = new Map(sessions.map((session) => [session.id, session]));
  const visible = visibleSessionId && byId.has(visibleSessionId) ? visibleSessionId : null;

  const next = previousIds.filter((id) => {
    if (!byId.has(id)) return false;
    return id === visible;
  });

  if (visible && !next.includes(visible)) {
    next.push(visible);
  }

  return next;
}

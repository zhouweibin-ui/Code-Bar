type RunnerSurfaceStatus = "idle" | "running" | "waiting" | "suspended" | "done" | "error";

interface RunnerSurfaceSession {
  status: RunnerSurfaceStatus;
  runner: { type: string };
  providerSessionId?: string;
}

function hasNativeResumeBinding(session: RunnerSurfaceSession | undefined): boolean {
  if (!session?.providerSessionId?.trim()) return false;
  return session.runner.type === "claude-code" || session.runner.type === "codex";
}

export function shouldAutoOpenRunnerSurface(session: RunnerSurfaceSession | undefined): boolean {
  if (!session) return false;
  return (
    session.status === "running"
    || session.status === "waiting"
    || session.status === "suspended"
    || hasNativeResumeBinding(session)
  );
}

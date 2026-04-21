import { invoke } from "@tauri-apps/api/core";
import { SplitDetailHost } from "../components/SplitSwapLayout";
import { ExploreEditor } from "../components/ExploreMode";
import { showSessionSurface, showExplorer, showScm } from "../services/workbenchCommands";
import { sanitizeRunnerConfig, useSettingsStore } from "../store/settingsStore";
import { useWorkspaceStore } from "../store/workspaceStore";
import { useWorkbenchStore } from "../store/workbenchStore";
import { useSessionStore, type ClaudeSession } from "../store/sessionStore";

function WelcomeAction({ label, accent = false, onClick }: { label: string; accent?: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        background: "none",
        border: "none",
        padding: "4px 2px",
        color: accent ? "var(--ci-accent)" : "var(--ci-text-muted)",
        fontSize: 12,
        fontWeight: 600,
        cursor: "pointer",
      }}
      onMouseEnter={e => {
        e.currentTarget.style.opacity = "0.8";
      }}
      onMouseLeave={e => {
        e.currentTarget.style.opacity = "1";
      }}
    >
      {label}
    </button>
  );
}

function WelcomeList({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <div style={{ fontSize: 10, color: "var(--ci-text-dim)", fontWeight: 700, letterSpacing: "0.06em", textTransform: "uppercase" }}>
        {title}
      </div>
      <div style={{ marginTop: 10 }}>
        {children}
      </div>
    </div>
  );
}

function WelcomeEntry({
  title,
  detail,
  action,
}: {
  title: string;
  detail?: string;
  action?: React.ReactNode;
}) {
  return (
    <div
      style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 18, padding: "10px 8px", borderTop: "1px solid var(--ci-toolbar-border)", transition: "background 0.12s" }}
      onMouseEnter={e => {
        e.currentTarget.style.background = "var(--ci-list-hover-bg)";
      }}
      onMouseLeave={e => {
        e.currentTarget.style.background = "transparent";
      }}
    >
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ fontSize: 13, fontWeight: 600, color: "var(--ci-text)" }}>{title}</div>
        {detail && <div style={{ marginTop: 3, fontSize: 11, color: "var(--ci-text-dim)", lineHeight: 1.6 }}>{detail}</div>}
      </div>
      {action && <div style={{ flexShrink: 0 }}>{action}</div>}
    </div>
  );
}

function WorkbenchWelcome({ session }: { session: ClaudeSession | null }) {
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const activeWorkspaceId = useWorkspaceStore((s) => s.activeWorkspaceId);
  const addWorkspace = useWorkspaceStore((s) => s.addWorkspace);
  const activeWorkspace = useWorkspaceStore((s) => s.workspaces.find((workspace) => workspace.id === s.activeWorkspaceId) ?? null);
  const sessions = useSessionStore((s) => s.sessions);
  const addSession = useSessionStore((s) => s.addSession);
  const setActiveSession = useSessionStore((s) => s.setActiveSession);
  const setExpandedSession = useSessionStore((s) => s.setExpandedSession);
  const markWorktreeReady = useSessionStore((s) => s.markWorktreeReady);
  const focusSession = useWorkbenchStore((s) => s.focusSession);
  const runner = sanitizeRunnerConfig(useSettingsStore((s) => s.settings.runner));
  const hasWorkspace = workspaces.length > 0;
  const recentSessions = activeWorkspace
    ? sessions.filter((item) => item.workspaceId === activeWorkspace.id).slice(0, 5)
    : [];
  const otherWorkspaces = workspaces.filter((workspace) => workspace.id !== activeWorkspaceId).slice(0, 4);

  const handleAddWorkspace = async () => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const picked = await invoke<string>("pick_folder").catch(() => "");
    const trimmed = picked.trim();
    if (!trimmed) return;
    const workspaceId = addWorkspace(trimmed);
    await invoke("clear_deleted_items", {
      sessionIds: [],
      workspaceIds: [],
      sessionRefs: [],
      workspaceRefs: [{ workspaceId, path: trimmed }],
    }).catch(() => {});
    await invoke("trust_workspace", { path: trimmed }).catch(() => {});
  };

  const handleNewSession = async () => {
    if (!activeWorkspace) return;

    let id: string;
    if ("__TAURI_INTERNALS__" in window) {
      try {
        id = await invoke<string>("reserve_session_id", {
          workspaces: workspaces.map((workspace) => ({
            workspaceId: workspace.id,
            workspacePath: workspace.path,
          })),
          existingSessionIds: sessions.map((item) => item.id),
        });
      } catch {
        return;
      }
    } else {
      const maxId = sessions
        .map((item) => Number(item.id))
        .filter((value) => !Number.isNaN(value))
        .reduce((max, value) => Math.max(max, value), 0);
      id = String(maxId + 1);
    }

    addSession(id, activeWorkspace.id, activeWorkspace.path, undefined, { ...runner });
    setActiveSession(id);
    setExpandedSession(id);
    focusSession(id);

    if ("__TAURI_INTERNALS__" in window) {
      await invoke("clear_deleted_items", {
        sessionIds: [id],
        workspaceIds: [],
        sessionRefs: [{ sessionId: id, workspaceId: activeWorkspace.id }],
        workspaceRefs: [],
      }).catch(() => {});
      await invoke("remember_session_workdir", {
        sessionId: id,
        workdir: activeWorkspace.path,
      }).catch(() => {});
      try {
        const result = await invoke<{
          worktree_path: string;
          branch: string;
          base_branch: string;
        } | null>("setup_session_worktree", {
          workdir: activeWorkspace.path,
          sessionId: id,
        });
        if (result) {
          await invoke("remember_session_workdir", {
            sessionId: id,
            workdir: result.worktree_path,
          }).catch(() => {});
          useSessionStore.getState().updateSession(id, {
            workdir: result.worktree_path,
            worktreePath: result.worktree_path,
            branchName: result.branch,
            baseBranch: result.base_branch,
          });
        }
      } catch {}
    }

    markWorktreeReady(id);
  };

  return (
    <div style={{ width: "100%", height: "100%", overflowY: "auto" }}>
      <div style={{ width: "100%", maxWidth: 760, margin: "0 auto", padding: "36px 30px 44px", boxSizing: "border-box" }}>
        <div style={{ fontSize: 10, color: "var(--ci-text-dim)", fontWeight: 700, letterSpacing: "0.08em", textTransform: "uppercase" }}>
          Get Started
        </div>
        <div style={{ marginTop: 8, fontSize: 26, fontWeight: 700, color: "var(--ci-text)", letterSpacing: -0.5, lineHeight: 1.15 }}>
          {hasWorkspace ? (activeWorkspace ? activeWorkspace.name : "Welcome") : "No workspace"}
        </div>
        <div style={{ marginTop: 8, maxWidth: 520, fontSize: 12, color: "var(--ci-text-muted)", lineHeight: 1.7 }}>
          {hasWorkspace ? "Choose where to continue." : "Add a project folder to begin."}
        </div>

        <div style={{ marginTop: 30, display: "grid", gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)", gap: 36 }}>
          {hasWorkspace ? (
            <>
              <WelcomeList title="Start">
                {session ? (
                  <>
                    <WelcomeEntry title="Open current session" detail="Continue the active terminal context." action={<WelcomeAction label="Open" accent onClick={() => showSessionSurface(session.id)} />} />
                    <WelcomeEntry title="Open Explorer" detail="Browse files and start editing." action={<WelcomeAction label="Explorer" onClick={() => showExplorer(session.id)} />} />
                    <WelcomeEntry title="Open Source Control" detail="Review current changes and conflicts." action={<WelcomeAction label="SCM" onClick={() => showScm(session.id)} />} />
                  </>
                ) : (
                  <WelcomeEntry
                    title="Create or choose a session"
                    detail="Use the sidebar on the left, or create one now."
                    action={<WelcomeAction label="New" accent onClick={() => { void handleNewSession(); }} />}
                  />
                )}
              </WelcomeList>
              <WelcomeList title="Recent">
                {recentSessions.length > 0 ? recentSessions.map((recent) => (
                  <WelcomeEntry
                    key={recent.id}
                    title={recent.name}
                    detail={recent.currentTask || undefined}
                    action={<WelcomeAction label="Open" onClick={() => showSessionSurface(recent.id)} />}
                  />
                )) : (
                  <WelcomeEntry title="No recent sessions" detail="New sessions will appear here." />
                )}
              </WelcomeList>
            </>
          ) : (
            <>
              <WelcomeList title="Start">
                <WelcomeEntry
                  title="Add workspace"
                  detail="Choose a project folder and make it the current workspace."
                  action={<WelcomeAction label="Add" accent onClick={() => { void handleAddWorkspace(); }} />}
                />
                <WelcomeEntry title="Create a session" detail="Available after a workspace is added." />
              </WelcomeList>
              <WelcomeList title="Overview">
                <WelcomeEntry title="Explorer and SCM" detail="Appear after a workspace is available." />
                <WelcomeEntry title="Split workbench" detail="Sidebar, center editor/detail, and widgets." />
              </WelcomeList>
            </>
          )}
        </div>

        {hasWorkspace && otherWorkspaces.length > 0 && (
          <div style={{ marginTop: 34, maxWidth: 520 }}>
            <div style={{ fontSize: 10, color: "var(--ci-text-dim)", fontWeight: 700, letterSpacing: "0.06em", textTransform: "uppercase" }}>
              Other Workspaces
            </div>
            <div style={{ marginTop: 10 }}>
              {otherWorkspaces.map((workspace) => {
                const workspaceSessions = sessions.filter((item) => item.workspaceId === workspace.id);
                const topSession = workspaceSessions[0] ?? null;
                return (
                  <div
                    key={workspace.id}
                    style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 14, padding: "8px 8px", borderTop: "1px solid var(--ci-toolbar-border)", transition: "background 0.12s" }}
                    onMouseEnter={e => {
                      e.currentTarget.style.background = "var(--ci-list-hover-bg)";
                    }}
                    onMouseLeave={e => {
                      e.currentTarget.style.background = "transparent";
                    }}
                  >
                    <div style={{ minWidth: 0, flex: 1 }}>
                      <div style={{ fontSize: 11.5, color: "var(--ci-text-muted)", fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {workspace.name}
                      </div>
                      {topSession && (
                        <div style={{ marginTop: 2, fontSize: 10, color: "var(--ci-text-dim)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                          {topSession.name}
                        </div>
                      )}
                    </div>
                    <WelcomeAction label="Switch" onClick={() => useWorkspaceStore.getState().bringToFront(workspace.id)} />
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export function WorkbenchCenter({
  session,
  onRefreshDiff,
}: {
  session: ClaudeSession | null;
  onRefreshDiff: (sessionId?: string | null, options?: { reloadExplorer?: boolean }) => void;
}) {
  const centerSurface = useWorkbenchStore((s) => s.centerSurface);

  if (centerSurface === "welcome") {
    return <WorkbenchWelcome session={session} />;
  }

  if (centerSurface === "editor" || centerSurface === "diff") {
    return <ExploreEditor session={session} onRefreshDiff={onRefreshDiff} />;
  }

  return <SplitDetailHost />;
}

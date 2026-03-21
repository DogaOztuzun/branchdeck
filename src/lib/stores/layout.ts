import { createSignal } from 'solid-js';
import type { PanelGroupAPI } from 'solid-resizable-panels';

export type RightPanelContext =
  | { kind: 'task'; worktreePath: string }
  | { kind: 'prs' }
  | { kind: 'changes' }
  | { kind: 'agents' };

export type AppView = 'workspace' | 'orchestrations';

const [panelApi, setPanelApi] = createSignal<PanelGroupAPI | null>(null);
const [repoSidebarOpen, setRepoSidebarOpen] = createSignal(true);
const [rightSidebarOpen, setRightSidebarOpen] = createSignal(true);
const [rightPanelContext, setRightPanelContext] = createSignal<RightPanelContext>({
  kind: 'agents',
});
const [activeView, setActiveView] = createSignal<AppView>('workspace');

// Track whether user explicitly set the panel context (vs auto-context)
const [userExplicitContext, setUserExplicitContext] = createSignal(false);

// Track last user-resized panel width
const [lastRightPanelSize, setLastRightPanelSize] = createSignal(18);

const REPO_SIDEBAR_ID = 'repo-sidebar';
const RIGHT_SIDEBAR_ID = 'right-sidebar';

function ensureRightPanelOpen() {
  const api = panelApi();
  if (!api) return;
  if (!rightSidebarOpen()) {
    api.expand(RIGHT_SIDEBAR_ID, lastRightPanelSize());
    setRightSidebarOpen(true);
  }
}

export function getLayoutStore() {
  return {
    setPanelApi,
    repoSidebarOpen,
    setRepoSidebarOpen,
    rightSidebarOpen,
    setRightSidebarOpen,
    rightPanelContext,
    activeView,
    setActiveView,
    lastRightPanelSize,
    setLastRightPanelSize,

    toggleRepoSidebar() {
      const api = panelApi();
      if (!api) return;
      if (repoSidebarOpen()) {
        api.collapse(REPO_SIDEBAR_ID);
        setRepoSidebarOpen(false);
      } else {
        api.expand(REPO_SIDEBAR_ID, 18);
        setRepoSidebarOpen(true);
      }
    },

    /** Explicit user action — always wins over auto-context */
    showRightPanel(ctx: RightPanelContext) {
      const current = rightPanelContext();
      // If same context, toggle the panel closed
      if (rightSidebarOpen() && current.kind === ctx.kind) {
        const api = panelApi();
        if (api) {
          api.collapse(RIGHT_SIDEBAR_ID);
          setRightSidebarOpen(false);
        }
        return;
      }
      setRightPanelContext(ctx);
      setUserExplicitContext(true);
      ensureRightPanelOpen();
    },

    /** Auto-context from worktree selection — resets explicit flag and sets context */
    autoContext(ctx: RightPanelContext) {
      setUserExplicitContext(false);
      setRightPanelContext(ctx);
      ensureRightPanelOpen();
    },

    /**
     * Atomic navigation: set view to workspace + select worktree + show task panel.
     * Used by Orchestrations drill-down.
     */
    navigateToTask(worktreePath: string) {
      setActiveView('workspace');
      setRightPanelContext({ kind: 'task', worktreePath });
      setUserExplicitContext(true);
      ensureRightPanelOpen();
    },

    // Keep backward-compat for toggleChangesSidebar (keyboard shortcut Ctrl+Shift+L)
    toggleChangesSidebar() {
      const current = rightPanelContext();
      const api = panelApi();
      if (!api) return;
      if (rightSidebarOpen() && current.kind === 'changes') {
        api.collapse(RIGHT_SIDEBAR_ID);
        setRightSidebarOpen(false);
      } else {
        setRightPanelContext({ kind: 'changes' });
        setUserExplicitContext(true);
        ensureRightPanelOpen();
      }
    },
  };
}

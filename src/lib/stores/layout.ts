import { createSignal } from 'solid-js';
import type { PanelGroupAPI } from 'solid-resizable-panels';

export type RightPanelContext =
  | { kind: 'task'; worktreePath: string }
  | { kind: 'changes' }
  | { kind: 'agents' };

export type AppView = 'workspace' | 'inbox' | 'sat' | 'tasks' | 'pr-triage' | 'lifecycle' | 'setup';

const [panelApi, setPanelApi] = createSignal<PanelGroupAPI | null>(null);
const [repoSidebarOpen, setRepoSidebarOpen] = createSignal(true);
const [rightSidebarOpen, setRightSidebarOpen] = createSignal(true);
const [rightPanelContext, setRightPanelContext] = createSignal<RightPanelContext>({
  kind: 'agents',
});
const [activeView, setActiveView] = createSignal<AppView>('workspace');

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

    /** Set right panel context. If same context and panel is open, toggles it closed. */
    showRightPanel(ctx: RightPanelContext) {
      const current = rightPanelContext();
      if (rightSidebarOpen() && current.kind === ctx.kind) {
        const api = panelApi();
        if (api) {
          api.collapse(RIGHT_SIDEBAR_ID);
          setRightSidebarOpen(false);
        }
        return;
      }
      setRightPanelContext(ctx);
      ensureRightPanelOpen();
    },

    /** Auto-context from worktree selection. Always sets context (last click wins). */
    autoContext(ctx: RightPanelContext) {
      setRightPanelContext(ctx);
      ensureRightPanelOpen();
    },

    /** Atomic navigation: workspace + worktree + task panel. Used by Orchestrations drill-down. */
    navigateToTask(worktreePath: string) {
      setActiveView('workspace');
      setRightPanelContext({ kind: 'task', worktreePath });
      ensureRightPanelOpen();
    },

    toggleChangesSidebar() {
      const current = rightPanelContext();
      const api = panelApi();
      if (!api) return;
      if (rightSidebarOpen() && current.kind === 'changes') {
        api.collapse(RIGHT_SIDEBAR_ID);
        setRightSidebarOpen(false);
      } else {
        setRightPanelContext({ kind: 'changes' });
        ensureRightPanelOpen();
      }
    },
  };
}

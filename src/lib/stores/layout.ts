import { createSignal } from 'solid-js';
import type { PanelGroupAPI } from 'solid-resizable-panels';

export type RightSidebarView = 'changes' | 'team';

const [panelApi, setPanelApi] = createSignal<PanelGroupAPI | null>(null);
const [repoSidebarOpen, setRepoSidebarOpen] = createSignal(true);
const [rightSidebarOpen, setRightSidebarOpen] = createSignal(true);
const [rightSidebarView, setRightSidebarView] = createSignal<RightSidebarView>('changes');

const REPO_SIDEBAR_ID = 'repo-sidebar';
const RIGHT_SIDEBAR_ID = 'right-sidebar';
const DEFAULT_SIDEBAR_SIZE = 18;

export function getLayoutStore() {
  return {
    setPanelApi,
    repoSidebarOpen,
    setRepoSidebarOpen,
    rightSidebarOpen,
    setRightSidebarOpen,
    rightSidebarView,
    toggleRepoSidebar() {
      const api = panelApi();
      if (!api) return;
      if (repoSidebarOpen()) {
        api.collapse(REPO_SIDEBAR_ID);
        setRepoSidebarOpen(false);
      } else {
        api.expand(REPO_SIDEBAR_ID, DEFAULT_SIDEBAR_SIZE);
        setRepoSidebarOpen(true);
      }
    },
    toggleChangesSidebar() {
      const api = panelApi();
      if (!api) return;
      if (rightSidebarOpen() && rightSidebarView() === 'changes') {
        api.collapse(RIGHT_SIDEBAR_ID);
        setRightSidebarOpen(false);
      } else {
        setRightSidebarView('changes');
        if (!rightSidebarOpen()) {
          api.expand(RIGHT_SIDEBAR_ID, DEFAULT_SIDEBAR_SIZE);
          setRightSidebarOpen(true);
        }
      }
    },
    toggleTeamSidebar() {
      const api = panelApi();
      if (!api) return;
      if (rightSidebarOpen() && rightSidebarView() === 'team') {
        api.collapse(RIGHT_SIDEBAR_ID);
        setRightSidebarOpen(false);
      } else {
        setRightSidebarView('team');
        if (!rightSidebarOpen()) {
          api.expand(RIGHT_SIDEBAR_ID, DEFAULT_SIDEBAR_SIZE);
          setRightSidebarOpen(true);
        }
      }
    },
    // Keep backward compat for existing code
    changesSidebarOpen: () => rightSidebarOpen() && rightSidebarView() === 'changes',
    teamSidebarOpen: () => rightSidebarOpen() && rightSidebarView() === 'team',
  };
}

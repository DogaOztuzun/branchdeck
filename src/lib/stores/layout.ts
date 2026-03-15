import { createSignal } from 'solid-js';
import type { PanelGroupAPI } from 'solid-resizable-panels';

const [panelApi, setPanelApi] = createSignal<PanelGroupAPI | null>(null);
const [repoSidebarOpen, setRepoSidebarOpen] = createSignal(true);
const [changesSidebarOpen, setChangesSidebarOpen] = createSignal(true);

const REPO_SIDEBAR_ID = 'repo-sidebar';
const CHANGES_SIDEBAR_ID = 'changes-sidebar';
const DEFAULT_SIDEBAR_SIZE = 18;

export function getLayoutStore() {
  return {
    setPanelApi,
    repoSidebarOpen,
    changesSidebarOpen,
    setRepoSidebarOpen,
    setChangesSidebarOpen,
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
      if (changesSidebarOpen()) {
        api.collapse(CHANGES_SIDEBAR_ID);
        setChangesSidebarOpen(false);
      } else {
        api.expand(CHANGES_SIDEBAR_ID, DEFAULT_SIDEBAR_SIZE);
        setChangesSidebarOpen(true);
      }
    },
  };
}

import { getLayoutStore } from './stores/layout';
import { getRepoStore } from './stores/repo';
import { getTerminalStore } from './stores/terminal';

function getWorktreePath(): string {
  const repoStore = getRepoStore();
  return repoStore.state.activeWorktreePath ?? repoStore.state.activeRepoPath ?? '';
}

export function registerShortcuts() {
  const terminalStore = getTerminalStore();
  const layout = getLayoutStore();

  document.addEventListener('keydown', (e) => {
    if (!e.ctrlKey || !e.shiftKey) return;

    const wtPath = getWorktreePath();

    switch (e.key) {
      case 'T':
        e.preventDefault();
        if (wtPath) terminalStore.openShellTab(wtPath);
        break;
      case 'A':
        e.preventDefault();
        if (wtPath) terminalStore.openClaudeTab(wtPath);
        break;
      case 'W': {
        e.preventDefault();
        const activeId = terminalStore.getActiveTabId(wtPath);
        if (activeId) terminalStore.closeTab(activeId);
        break;
      }
      case 'B':
        e.preventDefault();
        layout.toggleRepoSidebar();
        break;
      case 'L':
        e.preventDefault();
        layout.toggleChangesSidebar();
        break;
      case 'P':
        e.preventDefault();
        layout.showRightPanel({ kind: 'prs' });
        break;
    }
  });
}

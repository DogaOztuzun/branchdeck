import { register } from '@tauri-apps/plugin-global-shortcut';
import { getLayoutStore } from './stores/layout';
import { getRepoStore } from './stores/repo';
import { getTerminalStore } from './stores/terminal';

function getCwd(): string {
  const repoStore = getRepoStore();
  return repoStore.state.activeWorktree?.path ?? repoStore.state.activeRepo?.path ?? '.';
}

export async function registerShortcuts() {
  const terminalStore = getTerminalStore();
  const layout = getLayoutStore();

  await register('Ctrl+Shift+T', () => {
    terminalStore.openShellTab(getCwd());
  });

  await register('Ctrl+Shift+A', () => {
    terminalStore.openClaudeTab(getCwd());
  });

  await register('Ctrl+Shift+W', () => {
    const activeId = terminalStore.state.activeTabId;
    if (activeId) {
      terminalStore.closeTab(activeId);
    }
  });

  await register('Ctrl+Shift+B', () => {
    layout.toggleRepoSidebar();
  });

  await register('Ctrl+Shift+L', () => {
    layout.toggleChangesSidebar();
  });
}

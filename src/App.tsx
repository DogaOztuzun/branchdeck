import { Match, onMount, Show, Switch } from 'solid-js';
import { InboxView } from './components/inbox/InboxView';
import { Shell } from './components/layout/Shell';
import { TopBar } from './components/layout/TopBar';
import { LifecycleView } from './components/lifecycle/LifecycleView';
import { PrTriageView } from './components/pr/PrTriageView';
import { SATDashboard } from './components/sat/SATDashboard';
import { TaskBoard } from './components/task/TaskBoard';
import { CommandPalette } from './components/ui/CommandPalette';
import { ShortcutOverlay } from './components/ui/ShortcutOverlay';
import { ToastRegion } from './components/ui/ToastRegion';
import { registerShortcuts } from './lib/shortcuts';
import { getKeyboardStore } from './lib/stores/keyboard';
import { getLayoutStore } from './lib/stores/layout';
import type { ViewName } from './types/keyboard';

export function App() {
  const layout = getLayoutStore();
  const keyboard = getKeyboardStore();

  onMount(() => {
    registerShortcuts();
    keyboard.initFocusTracking();

    // Register global view-switching shortcuts
    const views: { key: string; view: ViewName }[] = [
      { key: '1', view: 'workspace' },
      { key: '2', view: 'inbox' },
      { key: '3', view: 'sat' },
      { key: '4', view: 'tasks' },
      { key: '5', view: 'lifecycle' },
    ];
    for (const v of views) {
      keyboard.registerShortcut({
        key: v.key,
        handler: () => layout.setActiveView(v.view),
        label: `Go to ${v.view.charAt(0).toUpperCase() + v.view.slice(1)}`,
        context: 'global',
        category: 'Navigation',
      });
    }

    // Global keydown dispatcher
    document.addEventListener('keydown', (e) => {
      const activeView = layout.activeView() as ViewName;
      // Skip if activeView is legacy pr-triage — treat as inbox
      const view = activeView === ('pr-triage' as string) ? 'inbox' : activeView;
      const handled = keyboard.dispatch(e.key, view as ViewName, e.metaKey, e.ctrlKey);
      if (handled) e.preventDefault();
    });
  });

  return (
    <div class="flex flex-col h-screen overflow-hidden bg-bg-main">
      <TopBar />
      <Switch>
        <Match when={layout.activeView() === 'workspace'}>
          <Shell />
        </Match>
        <Match when={layout.activeView() === 'inbox'}>
          <InboxView />
        </Match>
        <Match when={layout.activeView() === 'pr-triage'}>
          <PrTriageView />
        </Match>
        <Match when={layout.activeView() === 'sat'}>
          <SATDashboard />
        </Match>
        <Match when={layout.activeView() === 'tasks'}>
          <TaskBoard />
        </Match>
        <Match when={layout.activeView() === 'lifecycle'}>
          <LifecycleView />
        </Match>
      </Switch>
      <Show when={keyboard.isPaletteOpen()}>
        <CommandPalette />
      </Show>
      <Show when={keyboard.isOverlayOpen()}>
        <ShortcutOverlay />
      </Show>
      <ToastRegion />
    </div>
  );
}

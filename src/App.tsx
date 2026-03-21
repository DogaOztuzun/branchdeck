import { Match, onMount, Switch } from 'solid-js';
import { OrchestrationView } from './components/layout/OrchestrationView';
import { Shell } from './components/layout/Shell';
import { TopBar } from './components/layout/TopBar';
import { ApprovalBanner } from './components/task/ApprovalModal';
import { ToastRegion } from './components/ui/ToastRegion';
import { registerShortcuts } from './lib/shortcuts';
import { getLayoutStore } from './lib/stores/layout';

export function App() {
  const layout = getLayoutStore();

  onMount(() => {
    registerShortcuts();
  });

  return (
    <div class="flex flex-col h-screen overflow-hidden bg-bg-main">
      <TopBar />
      <Switch>
        <Match when={layout.activeView() === 'workspace'}>
          <Shell />
        </Match>
        <Match when={layout.activeView() === 'orchestrations'}>
          <OrchestrationView />
        </Match>
      </Switch>
      <ApprovalBanner />
      <ToastRegion />
    </div>
  );
}

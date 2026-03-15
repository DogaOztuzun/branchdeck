import { onMount } from 'solid-js';
import { Shell } from './components/layout/Shell';
import { TopBar } from './components/layout/TopBar';
import { registerShortcuts } from './lib/shortcuts';

export function App() {
  onMount(() => {
    registerShortcuts();
  });

  return (
    <div class="flex flex-col h-screen overflow-hidden">
      <TopBar />
      <Shell />
    </div>
  );
}

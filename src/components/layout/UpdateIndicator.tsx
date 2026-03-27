import { Match, Switch } from 'solid-js';
import { installUpdate } from '../../lib/commands/update';
import { getTaskStore } from '../../lib/stores/task';
import { getUpdateStore } from '../../lib/stores/update';
import { Button } from '../ui/Button';

export function UpdateIndicator() {
  const update = getUpdateStore();
  const taskStore = getTaskStore();

  const hasActiveRun = () => taskStore.state.activeRun !== null;

  return (
    <Switch>
      <Match when={update.status() === 'available'}>
        <span class="text-[11px] text-accent-info ml-2 inline-flex items-center gap-1.5">
          Update {update.version() ? `v${update.version()}` : ''} available
          <Button variant="ghost" size="compact" onClick={() => installUpdate()}>
            Install
          </Button>
        </span>
      </Match>
      <Match when={update.status() === 'downloading'}>
        <span class="text-[11px] text-accent-warning ml-2 animate-pulse-opacity">
          Downloading update...
        </span>
      </Match>
      <Match when={update.status() === 'ready' && hasActiveRun()}>
        <span class="text-[11px] text-accent-warning ml-2">
          Update ready — finish active runs first
        </span>
      </Match>
      <Match when={update.status() === 'ready'}>
        <span class="text-[11px] text-accent-success ml-2">
          Restart to update {update.version() ? `v${update.version()}` : ''}
        </span>
      </Match>
      <Match when={update.status() === 'checking'}>
        <span class="text-[11px] text-text-dim ml-2">Checking for updates...</span>
      </Match>
    </Switch>
  );
}

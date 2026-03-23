import { Match, Switch } from 'solid-js';
import { getLayoutStore } from '../../lib/stores/layout';
import { TaskDetail } from '../task/TaskDetail';
import { AgentsPanel } from './AgentsPanel';
import { ChangesSidebar } from './ChangesSidebar';

export function RightPanel() {
  const layout = getLayoutStore();

  return (
    <Switch fallback={<AgentsPanel />}>
      <Match when={layout.rightPanelContext().kind === 'task'}>
        {(() => {
          const ctx = () => layout.rightPanelContext();
          const worktreePath = () => (ctx().kind === 'task' ? ctx().worktreePath : '');
          return <TaskDetail worktreePath={worktreePath()} />;
        })()}
      </Match>
      <Match when={layout.rightPanelContext().kind === 'changes'}>
        <ChangesSidebar />
      </Match>
      <Match when={layout.rightPanelContext().kind === 'agents'}>
        <AgentsPanel />
      </Match>
    </Switch>
  );
}

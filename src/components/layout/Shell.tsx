import { Panel, PanelGroup, ResizeHandle } from 'solid-resizable-panels';
import { getLayoutStore } from '../../lib/stores/layout';
import { TerminalArea } from '../terminal/TerminalArea';
import { RepoSidebar } from './RepoSidebar';
import { RightPanel } from './RightPanel';

export function Shell() {
  const layout = getLayoutStore();

  return (
    <div class="flex-1 overflow-hidden" style={{ 'min-height': '0' }}>
      <PanelGroup direction="row" class="h-full" setAPI={layout.setPanelApi}>
        <Panel
          id="repo-sidebar"
          initialSize={18}
          minSize={12}
          collapsible
          class="h-full"
          onCollapse={() => layout.setRepoSidebarOpen(false)}
          onExpand={() => layout.setRepoSidebarOpen(true)}
        >
          <RepoSidebar />
        </Panel>
        <ResizeHandle class="w-1 bg-border-subtle hover:bg-accent-primary transition-colors cursor-col-resize" />
        <Panel id="terminal" initialSize={64} minSize={30} class="h-full">
          <TerminalArea />
        </Panel>
        <ResizeHandle class="w-1 bg-border-subtle hover:bg-accent-primary transition-colors cursor-col-resize" />
        <Panel
          id="right-sidebar"
          initialSize={18}
          minSize={12}
          collapsible
          class="h-full"
          onCollapse={() => layout.setRightSidebarOpen(false)}
          onExpand={() => layout.setRightSidebarOpen(true)}
        >
          <RightPanel />
        </Panel>
      </PanelGroup>
    </div>
  );
}

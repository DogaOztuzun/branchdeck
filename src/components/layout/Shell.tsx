import { Panel, PanelGroup, ResizeHandle } from 'solid-resizable-panels';
import { getLayoutStore } from '../../lib/stores/layout';
import { TerminalArea } from '../terminal/TerminalArea';
import { ChangesSidebar } from './ChangesSidebar';
import { RepoSidebar } from './RepoSidebar';
import { TeamSidebar } from './TeamSidebar';

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
        <ResizeHandle class="w-1 bg-border hover:bg-primary transition-colors cursor-col-resize" />
        <Panel id="terminal" initialSize={50} minSize={30} class="h-full">
          <TerminalArea />
        </Panel>
        <ResizeHandle class="w-1 bg-border hover:bg-primary transition-colors cursor-col-resize" />
        <Panel
          id="team-sidebar"
          initialSize={14}
          minSize={10}
          collapsible
          class="h-full"
          onCollapse={() => layout.setTeamSidebarOpen(false)}
          onExpand={() => layout.setTeamSidebarOpen(true)}
        >
          <TeamSidebar />
        </Panel>
        <ResizeHandle class="w-1 bg-border hover:bg-primary transition-colors cursor-col-resize" />
        <Panel
          id="changes-sidebar"
          initialSize={18}
          minSize={12}
          collapsible
          class="h-full"
          onCollapse={() => layout.setChangesSidebarOpen(false)}
          onExpand={() => layout.setChangesSidebarOpen(true)}
        >
          <ChangesSidebar />
        </Panel>
      </PanelGroup>
    </div>
  );
}

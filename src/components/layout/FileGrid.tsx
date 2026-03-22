import { createEffect, createMemo, createSignal, For, Show } from 'solid-js';
import { getRepoStatus } from '../../lib/commands/git';
import { getAgentStore } from '../../lib/stores/agent';
import { shortPath } from '../../lib/utils';
import type { FileAccess } from '../../types/agent';

type FileGridProps = {
  worktreePath: string;
};

type FileState = 'active' | 'modified' | 'read' | 'changed';

function dotColor(state: FileState): string {
  switch (state) {
    case 'active':
      return 'bg-accent-success';
    case 'modified':
      return 'bg-accent-warning';
    case 'read':
      return 'bg-accent-info';
    case 'changed':
      return 'bg-text-dim';
  }
}

function dotTextColor(state: FileState): string {
  switch (state) {
    case 'active':
      return 'text-accent-success';
    case 'modified':
      return 'text-accent-warning';
    case 'read':
      return 'text-accent-info';
    case 'changed':
      return 'text-text-dim';
  }
}

function stateLabel(state: FileState): string {
  switch (state) {
    case 'active':
      return 'Agent active';
    case 'modified':
      return 'Modified by agent';
    case 'read':
      return 'Read by agent';
    case 'changed':
      return 'Changed in branch';
  }
}

// Heat score determines dot size: 0=base, 1=medium, 2=large, 3=xl
function heatLevel(entry: FileEntry): number {
  if (!entry.access) return 0;
  let heat = 0;
  if (entry.access.accessCount >= 3) heat += 1;
  if (entry.access.accessCount >= 8) heat += 1;
  if (entry.agentCount > 1) heat += 1;
  if (entry.state === 'active') heat += 1;
  return Math.min(heat, 3);
}

const DOT_SIZES = ['w-2 h-2', 'w-2.5 h-2.5', 'w-3 h-3', 'w-3.5 h-3.5'];

type FileEntry = {
  path: string;
  state: FileState;
  access: FileAccess | null;
  gitStatus: string | null;
  agentCount: number;
};

export function FileGrid(props: FileGridProps) {
  const agentStore = getAgentStore();
  const [changedFiles, setChangedFiles] = createSignal<Map<string, string>>(new Map());
  const [hoveredFile, setHoveredFile] = createSignal<{
    entry: FileEntry;
    x: number;
    y: number;
  } | null>(null);

  // Track write events to trigger git status refresh
  const writeCount = createMemo(
    () =>
      agentStore.state.log.filter(
        (e) => e.kind === 'toolEnd' && (e.toolName === 'Write' || e.toolName === 'Edit'),
      ).length,
  );

  let fetchGeneration = 0;
  createEffect(() => {
    const wt = props.worktreePath;
    // Re-fetch when worktree changes or agent writes a file
    writeCount();
    if (!wt) return;
    const gen = ++fetchGeneration;
    getRepoStatus(wt)
      .then((statuses) => {
        if (gen !== fetchGeneration) return;
        const map = new Map<string, string>();
        for (const s of statuses) {
          map.set(s.path, s.status);
        }
        setChangedFiles(map);
      })
      .catch(() => {
        if (gen === fetchGeneration) setChangedFiles(new Map());
      });
  });

  const fileAccessMap = createMemo(() => {
    const map = new Map<string, FileAccess & { agents: Set<string> }>();
    for (const f of agentStore.state.log) {
      if (f.filePath) {
        const rel = f.filePath.startsWith(props.worktreePath)
          ? f.filePath.substring(props.worktreePath.length).replace(/^\//, '')
          : f.filePath;
        if (!map.has(rel)) {
          map.set(rel, {
            path: rel,
            lastTool: f.toolName ?? '',
            lastAgent: f.agentId ?? f.tabId ?? '',
            lastAccess: f.ts,
            accessCount: 1,
            wasModified: f.kind === 'toolEnd' && (f.toolName === 'Write' || f.toolName === 'Edit'),
            agents: new Set([f.tabId]),
          });
        } else {
          const existing = map.get(rel);
          if (existing) {
            existing.accessCount += 1;
            existing.lastAccess = f.ts;
            existing.agents.add(f.tabId);
            if (f.toolName) existing.lastTool = f.toolName;
            if (f.kind === 'toolEnd' && (f.toolName === 'Write' || f.toolName === 'Edit')) {
              existing.wasModified = true;
            }
          }
        }
      }
    }
    return map;
  });

  const activeFiles = createMemo(() => {
    const active = new Map<string, number>();
    for (const [, info] of Object.entries(agentStore.state.agentsByTab)) {
      if (info.currentFile) {
        const rel = info.currentFile.startsWith(props.worktreePath)
          ? info.currentFile.substring(props.worktreePath.length).replace(/^\//, '')
          : info.currentFile;
        active.set(rel, (active.get(rel) ?? 0) + 1);
      }
    }
    return active;
  });

  const entries = createMemo(() => {
    const result = new Map<string, FileEntry>();

    for (const [path, status] of changedFiles()) {
      result.set(path, { path, state: 'changed', access: null, gitStatus: status, agentCount: 0 });
    }

    for (const [path, access] of fileAccessMap()) {
      const activeCount = activeFiles().get(path) ?? 0;
      const isActive = activeCount > 0;
      const state: FileState = isActive ? 'active' : access.wasModified ? 'modified' : 'read';
      const agentCount = access.agents.size;
      const existing = result.get(path);
      if (existing) {
        existing.state = state;
        existing.access = access;
        existing.agentCount = agentCount;
      } else {
        result.set(path, { path, state, access, gitStatus: null, agentCount });
      }
    }

    return [...result.values()].sort((a, b) => {
      const order: Record<FileState, number> = { active: 0, modified: 1, read: 2, changed: 3 };
      const stateCompare = order[a.state] - order[b.state];
      if (stateCompare !== 0) return stateCompare;
      return heatLevel(b) - heatLevel(a) || a.path.localeCompare(b.path);
    });
  });

  return (
    <div class="overflow-y-auto max-h-52 p-2">
      <Show
        when={entries().length > 0}
        fallback={<div class="text-xs text-text-dim text-center py-2">No file activity</div>}
      >
        <div class="flex items-center justify-between mb-1.5 px-0.5">
          <span class="text-[10px] uppercase text-text-dim tracking-wider">Files</span>
          <span class="text-xs text-text-dim">{entries().length}</span>
        </div>
        <div class="flex flex-wrap gap-[3px] items-center">
          <For each={entries()}>
            {(entry) => {
              const heat = () => heatLevel(entry);
              return (
                <span
                  role="img"
                  aria-label={entry.path}
                  class={`inline-block rounded-sm transition-all duration-300 ${DOT_SIZES[heat()]} ${dotColor(entry.state)} ${entry.state === 'active' ? 'animate-pulse' : ''}`}
                  onMouseEnter={(e) => setHoveredFile({ entry, x: e.clientX, y: e.clientY })}
                  onMouseLeave={() => setHoveredFile(null)}
                />
              );
            }}
          </For>
        </div>

        {/* Tooltip */}
        <Show when={hoveredFile()}>
          {(hovered) => (
            <div
              class="fixed z-50 px-2 py-1.5 bg-bg-sidebar border border-border-subtle shadow-lg text-xs max-w-72 pointer-events-none"
              style={{
                left: `${hovered().x + 12}px`,
                top: `${hovered().y - 8}px`,
              }}
            >
              <div class="text-text-main truncate font-mono text-xs">
                {shortPath(hovered().entry.path, 2)}
              </div>
              <div class={`text-xs mt-0.5 ${dotTextColor(hovered().entry.state)}`}>
                {stateLabel(hovered().entry.state)}
              </div>
              <Show when={hovered().entry.gitStatus}>
                <div class="text-xs text-text-dim">Git: {hovered().entry.gitStatus}</div>
              </Show>
              <Show when={hovered().entry.access}>
                {(access) => (
                  <div class="text-xs text-text-dim mt-0.5">
                    {access().lastTool} ({access().accessCount}x)
                  </div>
                )}
              </Show>
              <Show when={hovered().entry.agentCount > 1}>
                <div class="text-xs text-accent-info mt-0.5">
                  {hovered().entry.agentCount} agents
                </div>
              </Show>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
}

import { createEffect, createMemo, createSignal, For, Show } from 'solid-js';
import { listRepoFiles } from '../../lib/commands/git';
import { getAgentStore } from '../../lib/stores/agent';
import type { FileAccess } from '../../types/agent';

type FileGridProps = {
  worktreePath: string;
  visible: boolean;
};

type FileDotStatus = 'idle' | 'read' | 'modified' | 'active';

function dotColor(status: FileDotStatus): string {
  switch (status) {
    case 'active':
      return 'bg-success';
    case 'modified':
      return 'bg-warning';
    case 'read':
      return 'bg-info';
    default:
      return 'bg-border';
  }
}

function getFileStatus(
  path: string,
  fileMap: Map<string, FileAccess>,
  activeFiles: Set<string>,
): FileDotStatus {
  if (activeFiles.has(path)) return 'active';
  const access = fileMap.get(path);
  if (!access) return 'idle';
  if (access.wasModified) return 'modified';
  return 'read';
}

function groupByDirectory(files: string[]): Map<string, string[]> {
  const groups = new Map<string, string[]>();
  for (const file of files) {
    const lastSlash = file.lastIndexOf('/');
    const dir = lastSlash >= 0 ? file.substring(0, lastSlash) : '.';
    const name = lastSlash >= 0 ? file.substring(lastSlash + 1) : file;
    if (!groups.has(dir)) groups.set(dir, []);
    groups.get(dir)?.push(name);
  }
  return groups;
}

export function FileGrid(props: FileGridProps) {
  const agentStore = getAgentStore();
  const [repoFiles, setRepoFiles] = createSignal<string[]>([]);
  const [hoveredFile, setHoveredFile] = createSignal<{ path: string; x: number; y: number } | null>(
    null,
  );

  createEffect(() => {
    if (props.worktreePath && props.visible) {
      listRepoFiles(props.worktreePath)
        .then((files) => setRepoFiles(files))
        .catch(() => setRepoFiles([]));
    }
  });

  const fileAccessMap = createMemo(() => {
    const map = new Map<string, FileAccess>();
    for (const f of agentStore.state.log) {
      if (f.filePath) {
        // Normalize: strip worktree prefix to get relative path
        const rel = f.filePath.startsWith(props.worktreePath)
          ? f.filePath.substring(props.worktreePath.length).replace(/^\//, '')
          : f.filePath;
        if (!map.has(rel)) {
          map.set(rel, {
            path: rel,
            lastTool: f.toolName ?? '',
            lastAgent: f.agentId ?? '',
            lastAccess: f.ts,
            accessCount: 1,
            wasModified: f.kind === 'toolEnd' && (f.toolName === 'Write' || f.toolName === 'Edit'),
          });
        } else {
          const existing = map.get(rel);
          if (existing) {
            existing.accessCount += 1;
            existing.lastAccess = f.ts;
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
    const active = new Set<string>();
    for (const [, info] of Object.entries(agentStore.state.agentsByTab)) {
      if (info.currentFile) {
        const rel = info.currentFile.startsWith(props.worktreePath)
          ? info.currentFile.substring(props.worktreePath.length).replace(/^\//, '')
          : info.currentFile;
        active.add(rel);
      }
    }
    return active;
  });

  const grouped = createMemo(() => groupByDirectory(repoFiles()));

  const hoveredAccess = createMemo(() => {
    const h = hoveredFile();
    if (!h) return null;
    return fileAccessMap().get(h.path) ?? null;
  });

  return (
    <Show when={props.visible}>
      <div class="h-full overflow-y-auto bg-bg p-3">
        <Show
          when={repoFiles().length > 0}
          fallback={
            <div class="text-xs text-text-muted text-center py-8">No files in repository index</div>
          }
        >
          <div class="space-y-2">
            <For each={[...grouped().entries()]}>
              {([dir, files]) => (
                <div>
                  <div class="text-[10px] text-text-muted mb-0.5 truncate" title={dir}>
                    {dir}
                  </div>
                  <div class="flex flex-wrap gap-[3px]">
                    <For each={files}>
                      {(file) => {
                        const fullPath = dir === '.' ? file : `${dir}/${file}`;
                        const status = () =>
                          getFileStatus(fullPath, fileAccessMap(), activeFiles());
                        return (
                          <span
                            role="img"
                            aria-label={fullPath}
                            class={`inline-block w-2.5 h-2.5 rounded-sm ${dotColor(status())}`}
                            title={fullPath}
                            onMouseEnter={(e) =>
                              setHoveredFile({ path: fullPath, x: e.clientX, y: e.clientY })
                            }
                            onMouseLeave={() => setHoveredFile(null)}
                          />
                        );
                      }}
                    </For>
                  </div>
                </div>
              )}
            </For>
          </div>

          {/* Tooltip */}
          <Show when={hoveredFile()}>
            {(hovered) => (
              <div
                class="fixed z-50 px-2 py-1.5 bg-surface border border-border rounded shadow-lg text-xs max-w-64 pointer-events-none"
                style={{
                  left: `${hovered().x + 12}px`,
                  top: `${hovered().y - 8}px`,
                }}
              >
                <div class="text-text truncate font-mono">{hovered().path}</div>
                <Show when={hoveredAccess()}>
                  {(access) => (
                    <div class="mt-1 space-y-0.5 text-[10px] text-text-muted">
                      <div>
                        Tool: <span class="text-text">{access().lastTool}</span>
                      </div>
                      <div>
                        Accesses: <span class="text-text">{access().accessCount}</span>
                      </div>
                      <Show when={access().wasModified}>
                        <div class="text-warning">Modified</div>
                      </Show>
                    </div>
                  )}
                </Show>
                <Show when={!hoveredAccess()}>
                  <div class="mt-0.5 text-[10px] text-text-muted">No agent activity</div>
                </Show>
              </div>
            )}
          </Show>
        </Show>
      </div>
    </Show>
  );
}

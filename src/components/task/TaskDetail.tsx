import { createEffect, createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import { cancelRun, launchRun, resumeRun, retryRun } from '../../lib/commands/run';
import { getAgentStore } from '../../lib/stores/agent';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore } from '../../lib/stores/task';
import { getTerminalStore } from '../../lib/stores/terminal';
import { parseArtifactSummary, statusColor } from '../../lib/utils';
import type { TaskInfo } from '../../types/task';
import { SectionHeader } from '../ui/SectionHeader';
import { CreateTaskModal } from './CreateTaskModal';
import { RunTimeline } from './RunTimeline';
import { TaskBadge } from './TaskBadge';

type TaskDetailProps = {
  worktreePath: string;
};

export function TaskDetail(props: TaskDetailProps) {
  const repoStore = getRepoStore();
  const layoutStore = getLayoutStore();
  const taskStore = getTaskStore();
  const agentStore = getAgentStore();
  const terminalStore = getTerminalStore();
  const [showCreateTask, setShowCreateTask] = createSignal(false);
  const [launchError, setLaunchError] = createSignal<string | null>(null);
  const [knowledgeExpanded, setKnowledgeExpanded] = createSignal(true);

  let launchErrorTimer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(launchErrorTimer));

  // Reset modal when worktree changes
  createEffect(() => {
    props.worktreePath;
    setShowCreateTask(false);
  });

  const task = createMemo(() => {
    const wt = props.worktreePath;
    if (!wt) return null;
    const key = wt.endsWith('/') ? wt : `${wt}/`;
    return taskStore.state.tasksByWorktree[key] ?? null;
  });

  const worktree = createMemo(() => {
    const activeRepo = repoStore.state.activeRepoPath;
    if (!activeRepo) return null;
    const wts = repoStore.state.worktreesByRepo[activeRepo] ?? [];
    return wts.find((wt) => wt.path === props.worktreePath) ?? null;
  });

  const canLaunch = createMemo(() => !taskStore.state.activeRun);

  function handleLaunch(t: TaskInfo) {
    if (!canLaunch()) return;
    clearTimeout(launchErrorTimer);
    setLaunchError(null);
    launchRun(t.path, props.worktreePath).catch((e) => {
      setLaunchError(String(e));
      launchErrorTimer = setTimeout(() => setLaunchError(null), 5000);
    });
  }

  const activeAgents = createMemo(() => {
    const visible = terminalStore.state.tabs.filter(
      (t) => t.type === 'claude' && t.worktreePath === props.worktreePath,
    );
    const agents = visible
      .map((tab) => ({
        tab: { id: tab.id, title: tab.title },
        agent: agentStore.getTabAgent(tab.id),
      }))
      .filter((a) => a.agent);

    const run = taskStore.state.activeRun;
    if (
      run?.tabId &&
      (run.status === 'running' || run.status === 'starting' || run.status === 'blocked')
    ) {
      const taskAgent = agentStore.getTabAgent(run.tabId);
      if (taskAgent) {
        agents.push({
          tab: { id: run.tabId, title: 'Task Agent' },
          agent: taskAgent,
        });
      }
    }

    return agents;
  });

  const knowledgeItems = createMemo(() => {
    const t = task();
    if (!t) return [];
    if (!t.body.includes('## Prior Knowledge') || t.body.includes('(none yet)')) return [];
    return (
      t.body
        .split('## Prior Knowledge')[1]
        ?.split('\n')
        .filter((l) => l.startsWith('- ')) ?? []
    );
  });

  return (
    <div class="flex flex-col h-full bg-bg-sidebar">
      {/* Header */}
      <div class="px-3 py-2 border-b border-border-subtle">
        <span class="text-[10px] font-bold uppercase text-text-dim tracking-wider">
          {task() ? 'Task' : 'Worktree'}
        </span>
      </div>

      <div class="flex-1 overflow-y-auto">
        <Show
          when={task()}
          fallback={
            <div class="px-3 py-4 text-center">
              <div class="text-base text-text-dim mb-3">No task for this worktree</div>
              <Show when={worktree()}>
                <button
                  type="button"
                  class="text-base text-accent-primary hover:text-accent-primary/80 cursor-pointer"
                  onClick={() => setShowCreateTask(true)}
                >
                  + Create Task
                </button>
              </Show>
            </div>
          }
        >
          {(t) => (
            <>
              {/* 1. Task Header */}
              <div class="px-3 py-2 border-b border-border-subtle">
                <div class="text-base text-text-main font-medium truncate">
                  {worktree()?.branch ?? 'unknown'}
                </div>
                <div class="flex items-center justify-between mt-1">
                  <span class="text-base text-text-dim">
                    {t().frontmatter.type === 'pr-shepherd' ? 'PR Shepherd' : 'Issue Fix'}
                  </span>
                  <TaskBadge status={t().frontmatter.status} />
                </div>
                {/* Metadata */}
                <div class="flex items-center gap-2 mt-1 text-base text-text-dim">
                  <Show when={t().frontmatter.pr}>
                    <span>PR #{t().frontmatter.pr}</span>
                  </Show>
                  <Show when={t().frontmatter['run-count'] > 0}>
                    <span>{t().frontmatter['run-count']} runs</span>
                  </Show>
                  <Show when={parseArtifactSummary(t().body)}>
                    {(artifacts) => (
                      <>
                        <Show when={artifacts().totalCommits > 0}>
                          <span>
                            {artifacts().totalCommits} commit
                            {artifacts().totalCommits === 1 ? '' : 's'}
                          </span>
                        </Show>
                        <Show when={artifacts().pr}>
                          <span class="text-accent-info">PR #{artifacts().pr}</span>
                        </Show>
                      </>
                    )}
                  </Show>
                </div>
                {/* Actions */}
                <div class="flex items-center gap-1.5 mt-2">
                  <Show when={canLaunch() && t().frontmatter.status === 'created'}>
                    <button
                      type="button"
                      class="px-2 py-1 text-base text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-primary cursor-pointer"
                      onClick={() => handleLaunch(t())}
                    >
                      Launch
                    </button>
                  </Show>
                  <Show when={taskStore.state.activeRun && t().frontmatter.status === 'running'}>
                    <button
                      type="button"
                      class="px-2 py-1 text-base text-red-400 hover:text-red-300 border border-border-subtle hover:border-red-400 cursor-pointer"
                      onClick={() => {
                        const sid = taskStore.state.activeRun?.sessionId;
                        if (sid) cancelRun(sid).catch(() => {});
                      }}
                    >
                      Cancel
                    </button>
                  </Show>
                  <Show
                    when={
                      canLaunch() &&
                      (t().frontmatter.status === 'failed' ||
                        t().frontmatter.status === 'cancelled')
                    }
                  >
                    <button
                      type="button"
                      class="px-2 py-1 text-base text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-primary cursor-pointer"
                      onClick={() => retryRun(t().path, props.worktreePath).catch(() => {})}
                    >
                      Retry
                    </button>
                    <button
                      type="button"
                      class="px-2 py-1 text-base text-text-dim hover:text-text-main border border-border-subtle hover:border-accent-info cursor-pointer"
                      onClick={() => resumeRun(t().path, props.worktreePath).catch(() => {})}
                    >
                      Resume
                    </button>
                  </Show>
                </div>
                <Show when={launchError()}>
                  <p class="mt-1 text-base text-accent-error truncate">{launchError()}</p>
                </Show>
              </div>

              {/* 2. PR Context (expanded by default for pr-shepherd) */}
              <Show when={t().frontmatter.type === 'pr-shepherd' && t().frontmatter.pr}>
                <div class="px-3 py-2 border-b border-border-subtle">
                  <div class="text-[10px] uppercase text-text-dim tracking-wider mb-1.5">
                    PR #{t().frontmatter.pr}
                  </div>
                  <div class="text-base space-y-1.5">
                    {/* Checks */}
                    <div class="flex items-center gap-2">
                      <span class="text-text-dim w-16 shrink-0">Checks</span>
                      <Show
                        when={t().body.includes('Failing checks:')}
                        fallback={<span class="text-accent-success">passing</span>}
                      >
                        <span class="text-accent-error">
                          {t()
                            .body.split('\n')
                            .find((l) => l.includes('Failing checks:'))
                            ?.replace('- Failing checks: ', '')
                            ?.replace('- ', '') ?? 'failing'}
                        </span>
                      </Show>
                    </div>
                    {/* Reviews */}
                    <div class="flex items-center gap-2">
                      <span class="text-text-dim w-16 shrink-0">Reviews</span>
                      <span class="text-text-main truncate">
                        {t()
                          .body.split('\n')
                          .find((l) => l.includes('Reviews:'))
                          ?.replace('- Reviews: ', '')
                          ?.replace('- ', '') ?? 'None'}
                      </span>
                    </div>
                    {/* Diff */}
                    <Show when={t().body.includes('Diff:')}>
                      <div class="flex items-center gap-2">
                        <span class="text-text-dim w-16 shrink-0">Diff</span>
                        {(() => {
                          const line =
                            t()
                              .body.split('\n')
                              .find((l) => l.includes('Diff:'))
                              ?.replace('- Diff: ', '')
                              ?.replace('- ', '') ?? '';
                          const addMatch = line.match(/\+(\d+)/);
                          const delMatch = line.match(/-(\d+)/);
                          const fileMatch = line.match(/(\d+)\s*(?:file|across)/);
                          return (
                            <span>
                              <Show when={addMatch}>
                                <span class="text-green-400">+{addMatch?.[1]}</span>
                              </Show>{' '}
                              <Show when={delMatch}>
                                <span class="text-red-400">-{delMatch?.[1]}</span>
                              </Show>
                              <Show when={fileMatch}>
                                <span class="text-text-dim"> {fileMatch?.[1]} files</span>
                              </Show>
                            </span>
                          );
                        })()}
                      </div>
                    </Show>
                  </div>
                </div>
              </Show>

              {/* 3. Knowledge (expanded by default when items exist) */}
              <Show when={knowledgeItems().length > 0}>
                <div class="border-b border-border-subtle">
                  <button
                    type="button"
                    class="flex items-center justify-between w-full px-3 py-1.5 text-[10px] uppercase text-text-dim tracking-wider hover:bg-bg-main/30 cursor-pointer"
                    onClick={() => setKnowledgeExpanded((v) => !v)}
                  >
                    <span>
                      Prior Knowledge{' '}
                      <span class="text-accent-info">({knowledgeItems().length})</span>
                    </span>
                    <span>{knowledgeExpanded() ? '\u25BC' : '\u25B6'}</span>
                  </button>
                  <Show when={knowledgeExpanded()}>
                    <div class="px-3 pb-2">
                      <div class="text-base text-text-dim space-y-0.5 bg-bg-main/30 px-2 py-1.5 border border-border-subtle/30 max-h-32 overflow-y-auto">
                        <For each={knowledgeItems()}>
                          {(line) => (
                            <div class="text-base leading-relaxed break-words">
                              {line.replace('- ', '')}
                            </div>
                          )}
                        </For>
                      </div>
                    </div>
                  </Show>
                </div>
              </Show>

              {/* 4. Agent Activity */}
              <Show when={activeAgents().length > 0}>
                <div class="border-b border-border-subtle">
                  <SectionHeader label="Active" count={activeAgents().length} />
                  <div class="mt-1 space-y-0.5">
                    <For each={activeAgents()}>
                      {(item) => (
                        <div class="flex items-center gap-2 px-3 py-1 text-base hover:bg-bg-main/30">
                          <span
                            class={`w-2 h-2 rounded-full shrink-0 ${statusColor(item.agent?.status ?? 'stopped')}`}
                          />
                          <div class="flex-1 min-w-0">
                            <div class="text-text-main truncate">{item.tab.title}</div>
                            <Show when={item.agent?.currentTool}>
                              <div class="text-base text-text-dim truncate">
                                {item.agent?.currentTool}
                                <Show when={item.agent?.currentFile}>
                                  {' '}
                                  <span class="opacity-60">{item.agent?.currentFile}</span>
                                </Show>
                              </div>
                            </Show>
                            <Show when={item.agent && item.agent.subagentCount > 0}>
                              <div class="text-base text-accent-info">
                                +{item.agent?.subagentCount} subagent
                                {item.agent?.subagentCount === 1 ? '' : 's'}
                              </div>
                            </Show>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </Show>

              {/* 5. Run Timeline */}
              <Show when={taskStore.state.runLog.length > 0 || taskStore.state.activeRun}>
                <RunTimeline
                  entries={taskStore.state.runLog}
                  visible={true}
                  activeRun={taskStore.state.activeRun}
                />
              </Show>

              {/* Approval handled by global ApprovalModal in App.tsx */}
            </>
          )}
        </Show>
      </div>

      {/* Agents link */}
      <Show when={worktree()}>
        <div class="p-2 border-t border-border-subtle">
          <button
            type="button"
            class="text-base text-text-dim hover:text-text-main cursor-pointer"
            onClick={() => layoutStore.showRightPanel({ kind: 'agents' })}
          >
            View Agents
          </button>
        </div>
      </Show>

      {/* Create Task Modal */}
      <Show when={worktree()}>
        {(wt) => (
          <CreateTaskModal
            open={showCreateTask()}
            worktreePath={wt().path}
            repo={repoStore.getActiveRepo()?.name ?? ''}
            branch={wt().branch}
            onClose={() => setShowCreateTask(false)}
          />
        )}
      </Show>
    </div>
  );
}

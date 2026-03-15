import { createStore, produce } from 'solid-js/store';
import type { FileStatus, RepoInfo, WorktreeInfo } from '../../types/git';
import {
  addRepository,
  createWorktree as createWorktreeCmd,
  getRepoStatus,
  listRepositories,
  listWorktrees,
  removeRepository,
} from '../commands/git';
import { getAppConfig, getRepoConfig, saveAppConfig, saveRepoConfig } from '../commands/workspace';

type RepoState = {
  repos: RepoInfo[];
  activeRepoPath: string | null;
  worktreesByRepo: Record<string, WorktreeInfo[]>;
  activeWorktreePath: string | null;
  statuses: FileStatus[];
};

function createRepoStore() {
  const [state, setState] = createStore<RepoState>({
    repos: [],
    activeRepoPath: null,
    worktreesByRepo: {},
    activeWorktreePath: null,
    statuses: [],
  });

  function getActiveRepo(): RepoInfo | null {
    if (!state.activeRepoPath) return null;
    return state.repos.find((r) => r.path === state.activeRepoPath) ?? null;
  }

  function getActiveWorktree(): WorktreeInfo | null {
    if (!state.activeRepoPath || !state.activeWorktreePath) return null;
    const wts = state.worktreesByRepo[state.activeRepoPath] ?? [];
    return wts.find((w) => w.path === state.activeWorktreePath) ?? null;
  }

  async function loadRepos() {
    const repos = await listRepositories();
    setState('repos', repos);
  }

  async function restoreLastSession() {
    await loadRepos();

    const config = await getAppConfig();
    if (!config.lastActiveRepo || state.repos.length === 0) return;

    const lastRepo = state.repos.find((r) => r.path === config.lastActiveRepo);
    if (!lastRepo) return;

    setState('activeRepoPath', lastRepo.path);
    const wts = await listWorktrees(lastRepo.path);
    setState('worktreesByRepo', lastRepo.path, wts);

    const repoConfig = await getRepoConfig(lastRepo.path);
    const targetWt = repoConfig.lastWorktree
      ? wts.find((w) => w.path === repoConfig.lastWorktree)
      : wts.find((w) => w.isMain);

    if (targetWt) {
      setActiveWorktree(targetWt.path);
    }
  }

  async function persistState() {
    try {
      const config = await getAppConfig();
      config.lastActiveRepo = state.activeRepoPath;
      await saveAppConfig(config);

      if (state.activeRepoPath && state.activeWorktreePath) {
        const repoConfig = await getRepoConfig(state.activeRepoPath);
        repoConfig.lastWorktree = state.activeWorktreePath;
        await saveRepoConfig(state.activeRepoPath, repoConfig);
      }
    } catch {
      // Config save is best-effort
    }
  }

  async function addRepo() {
    const repo = await addRepository();
    if (repo) {
      await loadRepos();
      await selectRepo(repo.path);
    }
  }

  async function removeRepo(repoPath: string) {
    await removeRepository(repoPath);
    setState(
      produce((s) => {
        s.repos = s.repos.filter((r) => r.path !== repoPath);
        delete s.worktreesByRepo[repoPath];
        if (s.activeRepoPath === repoPath) {
          s.activeRepoPath = null;
          s.activeWorktreePath = null;
          s.statuses = [];
        }
      }),
    );
    persistState();
  }

  async function ensureWorktreesLoaded(repoPath: string) {
    if (!state.worktreesByRepo[repoPath]) {
      const wts = await listWorktrees(repoPath);
      setState('worktreesByRepo', repoPath, wts);
    }
  }

  async function selectRepo(repoPath: string) {
    setState('activeRepoPath', repoPath);
    await ensureWorktreesLoaded(repoPath);
    const wts = state.worktreesByRepo[repoPath] ?? [];
    const main = wts.find((w) => w.isMain);
    if (main) {
      setActiveWorktree(main.path);
    } else {
      setState('activeWorktreePath', null);
      setState('statuses', []);
      persistState();
    }
  }

  async function selectRepoAndWorktree(repoPath: string, worktreePath: string) {
    setState('activeRepoPath', repoPath);
    setActiveWorktree(worktreePath);
  }

  async function setActiveWorktree(worktreePath: string) {
    setState('activeWorktreePath', worktreePath);
    await refreshStatus();
    persistState();
  }

  async function createWorktree(repoPath: string, name: string, branch?: string) {
    const wt = await createWorktreeCmd(repoPath, name, branch);
    setState(
      produce((s) => {
        const existing = s.worktreesByRepo[repoPath] ?? [];
        existing.push(wt);
        s.worktreesByRepo[repoPath] = existing;
      }),
    );
    return wt;
  }

  async function refreshStatus() {
    if (!state.activeWorktreePath) {
      setState('statuses', []);
      return;
    }
    const statuses = await getRepoStatus(state.activeWorktreePath);
    setState('statuses', statuses);
  }

  return {
    state,
    getActiveRepo,
    getActiveWorktree,
    loadRepos,
    restoreLastSession,
    ensureWorktreesLoaded,
    addRepo,
    removeRepo,
    selectRepo,
    selectRepoAndWorktree,
    createWorktree,
    refreshStatus,
  };
}

let store: ReturnType<typeof createRepoStore> | undefined;

export function getRepoStore() {
  if (!store) store = createRepoStore();
  return store;
}

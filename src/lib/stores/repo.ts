import { createStore, produce } from 'solid-js/store';
import type { FileStatus, RepoInfo, TrackingInfo, WorktreeInfo } from '../../types/git';
import type { PrInfo } from '../../types/github';
import { installAgentHooks, removeAgentHooks } from '../commands/agent';
import {
  addRepository,
  createWorktree as createWorktreeCmd,
  getBranchTracking,
  getRepoStatus,
  listRepositories,
  listWorktrees,
  removeRepository,
  removeWorktree as removeWorktreeCmd,
} from '../commands/git';
import { checkGithubAvailable, getPrStatus } from '../commands/github';
import { getAppConfig, getRepoConfig, saveAppConfig, saveRepoConfig } from '../commands/workspace';

type RepoState = {
  repos: RepoInfo[];
  activeRepoPath: string | null;
  worktreesByRepo: Record<string, WorktreeInfo[]>;
  activeWorktreePath: string | null;
  statuses: FileStatus[];
  trackingByBranch: Record<string, TrackingInfo | null>;
  prByBranch: Record<string, PrInfo | null>;
  githubAvailable: boolean;
};

function createRepoStore() {
  const [state, setState] = createStore<RepoState>({
    repos: [],
    activeRepoPath: null,
    worktreesByRepo: {},
    activeWorktreePath: null,
    statuses: [],
    trackingByBranch: {},
    prByBranch: {},
    githubAvailable: false,
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
    checkGithubAvailable().then((available) => setState('githubAvailable', available));
    await loadRepos();

    const config = await getAppConfig();
    if (!config.lastActiveRepo || state.repos.length === 0) return;

    const lastRepo = state.repos.find((r) => r.path === config.lastActiveRepo);
    if (!lastRepo) return;

    setState('activeRepoPath', lastRepo.path);
    installAgentHooks(lastRepo.path).catch(() => {});
    const wts = await listWorktrees(lastRepo.path);
    setState('worktreesByRepo', lastRepo.path, wts);
    loadBranchTracking(lastRepo.path);
    loadPrStatus(lastRepo.path);

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
    removeAgentHooks(repoPath).catch(() => {});
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

  async function loadBranchTracking(repoPath: string) {
    const wts = state.worktreesByRepo[repoPath] ?? [];
    const branches = wts.map((w) => w.branch).filter(Boolean);
    for (const branch of branches) {
      try {
        const tracking = await getBranchTracking(repoPath, branch);
        setState('trackingByBranch', branch, tracking);
      } catch {
        // Tracking is best-effort
      }
    }
  }

  async function refreshTracking() {
    if (state.activeRepoPath) {
      await loadBranchTracking(state.activeRepoPath);
    }
  }

  async function loadPrStatus(repoPath: string) {
    if (!state.githubAvailable) return;
    const wts = state.worktreesByRepo[repoPath] ?? [];
    const branches = wts.map((w) => w.branch).filter(Boolean);
    for (const branch of branches) {
      try {
        const pr = await getPrStatus(repoPath, branch);
        setState('prByBranch', branch, pr);
      } catch {
        // PR status is best-effort
      }
    }
  }

  async function refreshPrStatus() {
    if (state.activeRepoPath) {
      await loadPrStatus(state.activeRepoPath);
    }
  }

  async function ensureWorktreesLoaded(repoPath: string) {
    if (!state.worktreesByRepo[repoPath]) {
      const wts = await listWorktrees(repoPath);
      setState('worktreesByRepo', repoPath, wts);
    }
    loadBranchTracking(repoPath);
    loadPrStatus(repoPath);
  }

  async function selectRepo(repoPath: string) {
    // Remove hooks from previously active repo
    if (state.activeRepoPath && state.activeRepoPath !== repoPath) {
      removeAgentHooks(state.activeRepoPath).catch(() => {});
    }
    setState('activeRepoPath', repoPath);
    // Install hooks for newly selected repo
    installAgentHooks(repoPath).catch(() => {});
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
    if (state.activeRepoPath && state.activeRepoPath !== repoPath) {
      removeAgentHooks(state.activeRepoPath).catch(() => {});
    }
    setState('activeRepoPath', repoPath);
    installAgentHooks(repoPath).catch(() => {});
    setActiveWorktree(worktreePath);
  }

  async function setActiveWorktree(worktreePath: string) {
    setState('activeWorktreePath', worktreePath);
    await refreshStatus();
    persistState();
  }

  async function removeWorktree(repoPath: string, worktreeName: string, deleteBranch: boolean) {
    try {
      await removeWorktreeCmd(repoPath, worktreeName, deleteBranch);
    } catch (e) {
      throw new Error(String(e));
    }
    setState(
      produce((s) => {
        const wts = s.worktreesByRepo[repoPath];
        if (wts) {
          s.worktreesByRepo[repoPath] = wts.filter((w) => w.name !== worktreeName);
        }
        if (s.activeWorktreePath) {
          const stillExists = s.worktreesByRepo[repoPath]?.some(
            (w) => w.path === s.activeWorktreePath,
          );
          if (!stillExists) {
            const main = s.worktreesByRepo[repoPath]?.find((w) => w.isMain);
            s.activeWorktreePath = main?.path ?? null;
          }
        }
      }),
    );
    await refreshStatus();
    persistState();
  }

  async function createWorktree(
    repoPath: string,
    name: string,
    branch?: string,
    baseBranch?: string,
  ) {
    const wt = await createWorktreeCmd(repoPath, name, branch, baseBranch);
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
    removeWorktree,
    selectRepo,
    selectRepoAndWorktree,
    createWorktree,
    refreshStatus,
    loadBranchTracking,
    refreshTracking,
    loadPrStatus,
    refreshPrStatus,
  };
}

let store: ReturnType<typeof createRepoStore> | undefined;

export function getRepoStore() {
  if (!store) store = createRepoStore();
  return store;
}

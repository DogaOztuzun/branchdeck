describe('Layout structure', () => {
  it('should render the three-pane layout with resizable panels', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    const repoPanel = await $('[data-resizable-panel-id="repo-sidebar"]');
    const termPanel = await $('[data-resizable-panel-id="terminal"]');

    expect(await repoPanel.isExisting()).toBe(true);
    expect(await termPanel.isExisting()).toBe(true);
  });

  it('should show repo toggle and context panel buttons', async () => {
    const repoToggle = await $('button[aria-label="Toggle repositories"]');
    expect(await repoToggle.isExisting()).toBe(true);

    // New context buttons: PRs and Changes
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    const changesBtn = await $('button[aria-label="Toggle changes"]');
    expect(await prsBtn.isExisting()).toBe(true);
    expect(await changesBtn.isExisting()).toBe(true);
  });

  it('should show Workspace tab (Orchestrations conditional)', async () => {
    const workspaceBtn = await $('button*=Workspace');
    expect(await workspaceBtn.isExisting()).toBe(true);
    // Orchestrations tab only shows when queue active or permissions pending
  });

  it('should show the repo sidebar with Repositories header', async () => {
    const hasReposHeader = await browser.execute(() => {
      const el = document.querySelector('[data-resizable-panel-id="repo-sidebar"]');
      return el?.textContent?.toLowerCase().includes('repositories') ?? false;
    });
    expect(hasReposHeader).toBe(true);
  });

  it('should show Add Repository button', async () => {
    const addRepoBtn = await $('button*=Add Repository');
    expect(await addRepoBtn.isExisting()).toBe(true);
  });
});

describe('Layout structure', () => {
  it('should render the three-pane layout with resizable panels', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    // Panels use data-resizable-panel-id, not id
    const repoPanel = await $('[data-resizable-panel-id="repo-sidebar"]');
    const termPanel = await $('[data-resizable-panel-id="terminal"]');

    expect(await repoPanel.isExisting()).toBe(true);
    expect(await termPanel.isExisting()).toBe(true);
  });

  it('should show all four toggle buttons', async () => {
    const toggles = [
      'Toggle repositories',
      'Toggle team',
      'Toggle dashboard',
      'Toggle changes',
    ];

    for (const label of toggles) {
      const btn = await $(`button[aria-label="${label}"]`);
      expect(await btn.isExisting()).toBe(true);
    }
  });

  it('should show Workspace and Orchestrations view tabs', async () => {
    const workspaceBtn = await $('button*=Workspace');
    const orchBtn = await $('button*=Orchestrations');
    expect(await workspaceBtn.isExisting()).toBe(true);
    expect(await orchBtn.isExisting()).toBe(true);
  });

  it('should show the repo sidebar with Repositories header', async () => {
    // "REPOSITORIES" is uppercase via CSS (text-transform), so check case-insensitive
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

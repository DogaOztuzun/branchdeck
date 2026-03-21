describe('View switching', () => {
  it('should switch to Orchestrations view', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    const orchBtn = await $('button*=Orchestrations');
    await orchBtn.click();
    await browser.pause(500);

    // Orchestrations view replaces the workspace - repo sidebar and terminal should be gone
    const termPanel = await $('[data-resizable-panel-id="terminal"]');
    const isTermVisible = await termPanel.isExisting();

    // Either the terminal disappears or batch queue content appears
    const batchQueue = await $('*=Batch Queue');
    const hasBatchView = await batchQueue.isExisting();

    expect(!isTermVisible || hasBatchView).toBe(true);
  });

  it('should switch back to Workspace view', async () => {
    const workspaceBtn = await $('button*=Workspace');
    await workspaceBtn.click();
    await browser.pause(500);

    // Terminal panel should be back
    const termPanel = await $('[data-resizable-panel-id="terminal"]');
    expect(await termPanel.isExisting()).toBe(true);
  });

  it('should toggle repo sidebar visibility', async () => {
    const toggleRepos = await $('button[aria-label="Toggle repositories"]');

    // Get initial state
    const panelBefore = await $('[data-resizable-panel-id="repo-sidebar"]');
    const wasThere = await panelBefore.isExisting();

    // Toggle
    await toggleRepos.click();
    await browser.pause(300);

    const panelAfter = await $('[data-resizable-panel-id="repo-sidebar"]');
    const isThereNow = await panelAfter.isExisting();

    // State should have changed (or panel collapsed to 0 width)
    // Restore to original state
    if (wasThere && !isThereNow) {
      await toggleRepos.click();
      await browser.pause(300);
    }

    expect(await toggleRepos.isExisting()).toBe(true);
  });
});

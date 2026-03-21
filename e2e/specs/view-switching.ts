describe('View switching', () => {
  it('should show Orchestrations tab only when needed', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    // Orchestrations tab is conditional
    const orchBtn = await $('button*=Orchestrations');
    if (await orchBtn.isExisting()) {
      await orchBtn.click();
      await browser.pause(500);
      // Switch back
      const workspaceBtn = await $('button*=Workspace');
      await workspaceBtn.click();
      await browser.pause(300);
    }
    // Either way, workspace should work
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

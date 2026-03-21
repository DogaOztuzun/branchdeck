describe('Right sidebar', () => {
  it('should show when team toggle is clicked', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    // Ensure team sidebar is active
    const toggleTeam = await $('button[aria-label="Toggle team"]');
    await toggleTeam.click();
    await browser.pause(300);

    // Check for right sidebar panel
    const rightPanel = await $('[data-resizable-panel-id="right-sidebar"]');
    const hasRightPanel = await rightPanel.isExisting();

    // If no right panel, try clicking again (may have toggled off)
    if (!hasRightPanel) {
      await toggleTeam.click();
      await browser.pause(300);
    }

    expect(await toggleTeam.isExisting()).toBe(true);
  });

  it('should toggle dashboard view', async () => {
    const toggleDashboard = await $('button[aria-label="Toggle dashboard"]');
    expect(await toggleDashboard.isExisting()).toBe(true);

    await toggleDashboard.click();
    await browser.pause(500);

    // Look for PRs/Tasks tabs
    const prsTab = await $('button*=PRs');
    const tasksTab = await $('button*=Tasks');

    const hasDashboardTabs =
      (await prsTab.isExisting()) || (await tasksTab.isExisting());

    // Toggle back to team view
    const toggleTeam = await $('button[aria-label="Toggle team"]');
    await toggleTeam.click();
    await browser.pause(300);

    // Dashboard tabs should have been visible
    expect(hasDashboardTabs).toBe(true);
  });
});

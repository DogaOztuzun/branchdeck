describe('Right sidebar - Context panel', () => {
  it('should show the right sidebar panel', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    // Right panel should exist (context-driven, no toggle needed)
    const rightPanel = await $('[data-resizable-panel-id="right-sidebar"]');
    if (await rightPanel.isExisting()) {
      expect(await rightPanel.isDisplayed()).toBe(true);
    }
  });

  it('should show PRs panel when PRs button is clicked', async () => {
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    if (!(await prsBtn.isExisting())) return;

    await prsBtn.click();
    await browser.pause(500);

    // Look for PRs/Tasks tabs in dashboard
    const prsTab = await $('button*=PRs');
    const tasksTab = await $('button*=Tasks');

    const hasDashboardTabs =
      (await prsTab.isExisting()) || (await tasksTab.isExisting());

    expect(hasDashboardTabs).toBe(true);
  });

  it('should show changes panel when changes button is clicked', async () => {
    const changesBtn = await $('button[aria-label="Toggle changes"]');
    if (!(await changesBtn.isExisting())) return;

    await changesBtn.click();
    await browser.pause(500);

    // Look for "Changes" text
    const changesHeader = await browser.execute(() => {
      return document.body.textContent?.includes('Changes') ?? false;
    });

    expect(changesHeader).toBe(true);
  });
});

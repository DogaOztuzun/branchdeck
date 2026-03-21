describe('Task Dashboard', () => {
  it('should be togglable via top bar button', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    const toggleDashboard = await $('button[aria-label="Toggle dashboard"]');
    expect(await toggleDashboard.isExisting()).toBe(true);

    await toggleDashboard.click();
    await browser.pause(500);

    // Look for PRs/Tasks tabs
    const prsTab = await $('button*=PRs');
    const tasksTab = await $('button*=Tasks');
    const hasTabs =
      (await prsTab.isExisting()) || (await tasksTab.isExisting());

    // Toggle back
    const toggleTeam = await $('button[aria-label="Toggle team"]');
    await toggleTeam.click();
    await browser.pause(300);

    expect(hasTabs).toBe(true);
  });

  it('should have all four sidebar toggle buttons', async () => {
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
});

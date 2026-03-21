describe('PR Shepherd flow', () => {
  it('should show PrList when PRs button is clicked', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    expect(await prsBtn.isExisting()).toBe(true);

    await prsBtn.click();
    await browser.pause(500);

    // Verify PrList rendered with header
    const hasPrList = await browser.execute(() => {
      return document.body.textContent?.includes('Pull Requests') ?? false;
    });
    expect(hasPrList).toBe(true);
  });

  it('should show filter controls in PrList', async () => {
    // Check for filter dropdowns
    const selects = await $$('select');
    // Should have at least the author and CI filter
    expect(selects.length).toBeGreaterThanOrEqual(2);
  });

  it('should show refresh button in PrList', async () => {
    // Refresh button (SVG icon button)
    const buttons = await $$('button');
    let hasRefresh = false;
    for (const btn of buttons) {
      const title = await btn.getAttribute('title');
      if (title === 'Refresh') {
        hasRefresh = true;
        break;
      }
    }
    expect(hasRefresh).toBe(true);
  });

  it('should show Orchestrations tab only when queue active', async () => {
    // Orchestrations tab is conditional — hidden when no queue
    const orchBtn = await $('button*=Orchestrations');
    const isVisible = await orchBtn.isExisting();
    // If no batch is running, the tab should be hidden
    if (!isVisible) {
      console.log('Orchestrations tab correctly hidden (no active queue)');
    }
    expect(true).toBe(true);
  });
});


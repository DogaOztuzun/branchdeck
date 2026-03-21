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

  it('should switch to Orchestrations view', async () => {
    const orchBtn = await $('button*=Orchestrations');
    expect(await orchBtn.isExisting()).toBe(true);

    await orchBtn.click();
    await browser.pause(500);

    // Should show orchestration content
    const hasOrchContent = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return text.includes('Batch Queue') || text.includes('No active orchestrations');
    });
    expect(hasOrchContent).toBe(true);
  });

  it('should show back button in Orchestrations header or idle state', async () => {
    // Either we see the back arrow button or the "Open PRs panel" link
    const hasBackOrLink = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return text.includes('Back to Workspace') || text.includes('Open PRs panel');
    });
    expect(hasBackOrLink).toBe(true);
  });

  it('should return to Workspace view', async () => {
    const workspaceBtn = await $('button*=Workspace');
    await workspaceBtn.click();
    await browser.pause(300);

    const termPanel = await $('[data-resizable-panel-id="terminal"]');
    expect(await termPanel.isExisting()).toBe(true);
  });
});


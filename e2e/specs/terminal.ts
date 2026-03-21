describe('Terminal area', () => {
  it('should show the terminal panel', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    const termPanel = await $('[data-resizable-panel-id="terminal"]');
    expect(await termPanel.isExisting()).toBe(true);
  });

  it('should show terminal action buttons (Open Terminal / Start Claude Code)', async () => {
    const openTermBtn = await $('button*=Open Terminal');
    const claudeBtn = await $('button*=Start Claude Code');

    // Either action buttons or an active terminal tab should exist
    const hasActions =
      (await openTermBtn.isExisting()) || (await claudeBtn.isExisting());
    expect(hasActions).toBe(true);
  });

  it('should have a plus (+) button for new tabs', async () => {
    // The plus button exists in the terminal tab bar
    const buttons = await $$('button');
    let foundPlus = false;
    for (const btn of buttons) {
      const text = await btn.getText();
      if (text.trim() === '+') {
        foundPlus = true;
        break;
      }
    }
    expect(foundPlus).toBe(true);
  });
});

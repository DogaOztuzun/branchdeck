describe('Task Dashboard', () => {
  it('should be accessible via PRs button in top bar', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    // New TopBar has PRs button instead of dashboard toggle
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    expect(await prsBtn.isExisting()).toBe(true);
  });

  it('should have PRs and Changes buttons in top bar', async () => {
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    const changesBtn = await $('button[aria-label="Toggle changes"]');

    expect(await prsBtn.isExisting()).toBe(true);
    expect(await changesBtn.isExisting()).toBe(true);
  });
});

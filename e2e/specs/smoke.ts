describe('Branchdeck smoke test', () => {
  it('should launch and show the main window', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );
    expect(await browser.getTitle()).toBe('Branchdeck');
  });

  it('should render the app root with content', async () => {
    const root = await $('#root');
    await root.waitForExist({ timeout: 15000 });
    const children = await $$('#root > *');
    expect(children.length).toBeGreaterThan(0);
  });

  it('should show the Branchdeck branding', async () => {
    const hasBranding = await browser.execute(() => {
      return document.body.textContent?.includes('Branchdeck') ?? false;
    });
    expect(hasBranding).toBe(true);
  });
});

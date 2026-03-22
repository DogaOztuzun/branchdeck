describe('Font size check', () => {
  it('should verify font sizes', async () => {
    await browser.waitUntil(async () => (await browser.getTitle()) === 'Branchdeck', { timeout: 15000 });

    const sizes = await browser.execute(() => {
      const results: Record<string, string> = {};
      // Repo sidebar items
      const repoButtons = document.querySelectorAll('[data-resizable-panel-id="repo-sidebar"] button');
      for (let i = 0; i < Math.min(repoButtons.length, 5); i++) {
        const btn = repoButtons[i] as HTMLElement;
        results[`repo-btn-${i}: "${btn.textContent?.trim().substring(0, 30)}"`] = getComputedStyle(btn).fontSize;
      }
      // Repo sidebar spans
      const repoSpans = document.querySelectorAll('[data-resizable-panel-id="repo-sidebar"] span');
      for (let i = 0; i < Math.min(repoSpans.length, 8); i++) {
        const span = repoSpans[i] as HTMLElement;
        const text = span.textContent?.trim().substring(0, 20);
        if (text) results[`repo-span-${i}: "${text}"`] = getComputedStyle(span).fontSize;
      }
      // TopBar
      const topbar = document.querySelector('.bg-bg-sidebar');
      if (topbar) {
        const spans = topbar.querySelectorAll('span');
        for (let i = 0; i < Math.min(spans.length, 5); i++) {
          const text = (spans[i] as HTMLElement).textContent?.trim().substring(0, 20);
          if (text) results[`topbar: "${text}"`] = getComputedStyle(spans[i] as HTMLElement).fontSize;
        }
      }
      return results;
    });

    console.log('=== FONT SIZES ===');
    for (const [key, val] of Object.entries(sizes)) {
      console.log(`  ${val}  ${key}`);
    }

    await browser.saveScreenshot('.gstack/design-reports/screenshots/font-check.png');
  });
});

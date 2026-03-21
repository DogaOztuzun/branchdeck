describe('Drive app', () => {
  it('interactive session', async () => {
    // Wait for app
    await browser.waitUntil(async () => (await browser.getTitle()) === 'Branchdeck', { timeout: 15000 });
    console.log('APP LOADED');

    // 1. Open PRs panel
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    await prsBtn.click();
    console.log('CLICKED PRs button');

    // 2. Wait for PRs to fully render
    await browser.waitUntil(async () => {
      return await browser.execute(() => document.body.textContent?.includes('Shepherd') ?? false);
    }, { timeout: 60000, interval: 2000 });

    // Dump PR list
    const prList = await browser.execute(() => {
      const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      return panel?.textContent ?? '';
    });
    console.log('PR LIST:', prList.substring(0, 500));

    // 3. Find PR #1 shepherd button (fix: change greeting prefix)
    const clickedPr1 = await browser.execute(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      // Find all Shepherd buttons, match the one near "fix: change greeting" or "#1"
      const allShepherds = btns.filter(b => b.textContent?.trim() === 'Shepherd' && !b.disabled);
      // Click the second one (PR #1 for alpha repo) - first is PR #2
      if (allShepherds.length >= 2) {
        (allShepherds[1] as HTMLButtonElement).click();
        return 'clicked second Shepherd (PR #1)';
      }
      if (allShepherds.length === 1) {
        (allShepherds[0] as HTMLButtonElement).click();
        return 'clicked only Shepherd button';
      }
      return 'no Shepherd buttons found';
    });
    console.log('SHEPHERD:', clickedPr1);

    // 4. Wait for navigation
    await browser.waitUntil(async () => {
      return await browser.execute(() => {
        const text = document.body.textContent ?? '';
        return text.includes('pr shepherd') || text.includes('Shepherd failed') || text.includes('Agents');
      });
    }, { timeout: 30000, interval: 1000 });

    // 5. Dump what we see
    const afterShepherd = await browser.execute(() => {
      const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      return {
        rightPanel: panel?.textContent?.substring(0, 1000) ?? 'NONE',
        allButtons: Array.from(document.querySelectorAll('button')).map(b => b.textContent?.trim().substring(0, 40)).filter(t => t && t.length > 0),
      };
    });
    console.log('RIGHT PANEL:', afterShepherd.rightPanel.substring(0, 600));
    console.log('BUTTONS:', afterShepherd.allButtons.join(' | '));

    // 6. Check if there's a Launch/Retry button and click it
    const hasLaunch = await browser.execute(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(
        b => b.textContent?.trim() === 'Launch' || b.textContent?.trim() === 'Retry'
      );
      if (btn) {
        const text = btn.textContent?.trim();
        (btn as HTMLButtonElement).click();
        return `clicked ${text}`;
      }
      return 'no Launch/Retry button';
    });
    console.log('LAUNCH:', hasLaunch);

    if (hasLaunch.includes('no ')) {
      console.log('No actionable button - task may already be running or completed');
      return;
    }

    // 7. Wait for run to start and check for permission dialog
    await browser.pause(5000);

    const runState = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return {
        hasPermission: text.includes('Permission Required'),
        hasRunning: text.includes('running') || text.includes('Running'),
        hasApprove: text.includes('Approve'),
        snippet: text.substring(0, 800),
      };
    });
    console.log('RUN STATE - Permission:', runState.hasPermission, '| Running:', runState.hasRunning, '| Approve:', runState.hasApprove);
    if (runState.hasPermission) {
      console.log('PERMISSION DIALOG FOUND');
    }

    // 8. If permission dialog, approve it
    if (runState.hasApprove) {
      await browser.execute(() => {
        const btn = Array.from(document.querySelectorAll('button')).find(
          b => b.textContent?.trim() === 'Approve'
        );
        (btn as HTMLButtonElement)?.click();
      });
      console.log('CLICKED Approve');
      await browser.pause(3000);
    }

    // 9. Check orchestrations
    const orchBtn = await $('button*=Orchestrations');
    await orchBtn.click();
    await browser.pause(2000);

    const orchState = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return text.substring(0, 600);
    });
    console.log('ORCHESTRATIONS:', orchState);

    // 10. Go back
    const wsBtn = await $('button*=Workspace');
    await wsBtn.click();
    await browser.pause(500);

    // Final state
    const finalPanel = await browser.execute(() => {
      const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      return panel?.textContent?.substring(0, 500) ?? 'NONE';
    });
    console.log('FINAL RIGHT PANEL:', finalPanel);
  });
});

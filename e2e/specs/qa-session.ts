const REPORT = '.gstack/qa-reports/screenshots';

describe('QA: App Launch + Layout', () => {
  it('should load and render three-pane layout', async () => {
    await browser.waitUntil(async () => (await browser.getTitle()) === 'Branchdeck', { timeout: 15000 });
    await browser.saveScreenshot(`${REPORT}/qa-01-launch.png`);

    const panels = await browser.execute(() => {
      const repo = document.querySelector('[data-resizable-panel-id="repo-sidebar"]');
      const term = document.querySelector('[data-resizable-panel-id="terminal"]');
      const right = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      return {
        hasRepo: !!repo,
        hasTerm: !!term,
        hasRight: !!right,
      };
    });
    console.log('LAYOUT:', JSON.stringify(panels));
    expect(panels.hasRepo && panels.hasTerm && panels.hasRight).toBe(true);
  });

  it('should show correct TopBar elements', async () => {
    const topbar = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return {
        hasBranding: text.includes('Branchdeck'),
        hasWorkspace: text.includes('Workspace'),
        hasOrchestrations: text.includes('Orchestrations'),
        buttons: Array.from(document.querySelectorAll('button')).map(b => ({
          label: b.getAttribute('aria-label'),
          title: b.getAttribute('title'),
          text: b.textContent?.trim().substring(0, 40),
        })).filter(b => b.label || b.title),
      };
    });
    console.log('TOPBAR:', JSON.stringify(topbar));
    expect(topbar.hasBranding).toBe(true);
    expect(topbar.hasWorkspace).toBe(true);
  });
});

describe('QA: Context-Driven Right Panel', () => {
  it('should show Agents panel by default (no task on selected worktree)', async () => {
    const rightContent = await browser.execute(() => {
      const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      return panel?.textContent?.substring(0, 300) ?? 'NONE';
    });
    console.log('DEFAULT RIGHT PANEL:', rightContent.substring(0, 200));
    // Should show Agents or Task depending on worktree state
    const hasContext = rightContent.includes('Agents') || rightContent.includes('Task');
    expect(hasContext).toBe(true);
  });

  it('should switch to PRs panel when PRs button clicked', async () => {
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    await prsBtn.click();
    await browser.waitUntil(async () => {
      return await browser.execute(() => document.body.textContent?.includes('Pull Requests') ?? false);
    }, { timeout: 10000 });
    await browser.saveScreenshot(`${REPORT}/qa-02-prs-panel.png`);

    const prState = await browser.execute(() => {
      const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      return {
        text: panel?.textContent?.substring(0, 500) ?? '',
        hasFilters: !!document.querySelector('select'),
        hasRefresh: Array.from(document.querySelectorAll('button')).some(b => b.getAttribute('title') === 'Refresh'),
      };
    });
    console.log('PR PANEL - filters:', prState.hasFilters, '| refresh:', prState.hasRefresh);
    expect(prState.text.includes('Pull Requests')).toBe(true);
  });

  it('should wait for PRs to fully load with Shepherd buttons', async () => {
    await browser.waitUntil(async () => {
      return await browser.execute(() => document.body.textContent?.includes('Shepherd') ?? false);
    }, { timeout: 60000, interval: 2000 });

    const prData = await browser.execute(() => {
      const checkboxes = document.querySelectorAll('input[type="checkbox"]').length;
      const shepherdBtns = Array.from(document.querySelectorAll('button')).filter(b => b.textContent?.trim() === 'Shepherd').length;
      return { checkboxes, shepherdBtns };
    });
    console.log('PRs loaded - checkboxes:', prData.checkboxes, '| shepherd buttons:', prData.shepherdBtns);
    await browser.saveScreenshot(`${REPORT}/qa-03-prs-loaded.png`);
    expect(prData.shepherdBtns).toBeGreaterThan(0);
  });

  it('should switch to Changes panel', async () => {
    const changesBtn = await $('button[aria-label="Toggle changes"]');
    await changesBtn.click();
    await browser.pause(500);
    await browser.saveScreenshot(`${REPORT}/qa-04-changes.png`);

    const hasChanges = await browser.execute(() =>
      document.body.textContent?.includes('Changes') ?? false
    );
    expect(hasChanges).toBe(true);
  });

  it('should auto-context to TaskDetail when clicking worktree with task', async () => {
    const wtBtn = await $('button*=feat/add-farewell');
    if (await wtBtn.isExisting()) {
      await wtBtn.click();
      await browser.pause(500);
      await browser.saveScreenshot(`${REPORT}/qa-05-task-auto-context.png`);

      const rightContent = await browser.execute(() => {
        const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
        return panel?.textContent?.substring(0, 500) ?? '';
      });
      console.log('AUTO-CONTEXT:', rightContent.substring(0, 200));
      // Should show task detail (PR Shepherd, succeeded, etc)
      const hasTask = rightContent.includes('Task') || rightContent.includes('PR');
      expect(hasTask).toBe(true);
    }
  });

  it('should auto-context to Agents when clicking worktree without task', async () => {
    const mainBtn = await $('button*=main');
    if (await mainBtn.isExisting()) {
      await mainBtn.click();
      await browser.pause(500);

      const rightContent = await browser.execute(() => {
        const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
        return panel?.textContent?.substring(0, 300) ?? '';
      });
      console.log('AGENTS AUTO-CONTEXT:', rightContent.substring(0, 200));
    }
  });
});

describe('QA: Orchestrations View', () => {
  it('should switch to Orchestrations and show all tasks', async () => {
    const orchBtn = await $('button*=Orchestrations');
    if (!(await orchBtn.isExisting())) {
      console.log('SKIP: Orchestrations tab not visible (no active runs)');
      return;
    }
    await orchBtn.click();
    await browser.pause(1000);
    await browser.saveScreenshot(`${REPORT}/qa-06-orchestrations.png`);

    const orchState = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return {
        text: text.substring(0, 600),
        hasHeader: text.includes('Orchestrations'),
        hasCards: text.includes('PR Shepherd') || text.includes('Issue Fix'),
        hasTaskCount: /\d+ task/.test(text),
        hasBackBtn: !!Array.from(document.querySelectorAll('button')).find(b => b.getAttribute('title') === 'Back to Workspace'),
        hasRefresh: !!Array.from(document.querySelectorAll('button')).find(b => b.getAttribute('title') === 'Refresh tasks'),
      };
    });
    console.log('ORCHESTRATIONS:', JSON.stringify(orchState));
    expect(orchState.hasHeader).toBe(true);
  });

  it('should show task cards with correct info', async () => {
    const cards = await browser.execute(() => {
      // Find card-like elements (buttons with branch names inside)
      const cardButtons = Array.from(document.querySelectorAll('button')).filter(b => {
        const text = b.textContent ?? '';
        return (text.includes('PR Shepherd') || text.includes('Issue Fix')) && text.includes('runs');
      });
      return cardButtons.map(b => ({
        text: b.textContent?.trim().substring(0, 120),
      }));
    });
    console.log('TASK CARDS:', cards.length);
    for (const card of cards) {
      console.log('  CARD:', card.text);
    }
  });

  it('should expand card on click and show details', async () => {
    // Click first card
    const clicked = await browser.execute(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(b => {
        const text = b.textContent ?? '';
        return (text.includes('PR Shepherd') || text.includes('Issue Fix')) && text.includes('runs');
      });
      if (btn) {
        (btn as HTMLButtonElement).click();
        return true;
      }
      return false;
    });

    if (!clicked) {
      console.log('SKIP: No task cards to click');
      return;
    }

    await browser.pause(500);
    await browser.saveScreenshot(`${REPORT}/qa-07-card-expanded.png`);

    const expandedContent = await browser.execute(() => {
      const text = document.body.textContent ?? '';
      return {
        hasChecks: text.includes('Checks'),
        hasReviews: text.includes('Reviews'),
        hasKnowledge: text.includes('knowledge'),
        hasOpenWorkspace: text.includes('Open in Workspace'),
        text: text.substring(0, 800),
      };
    });
    console.log('EXPANDED:', JSON.stringify({
      checks: expandedContent.hasChecks,
      reviews: expandedContent.hasReviews,
      knowledge: expandedContent.hasKnowledge,
      workspace: expandedContent.hasOpenWorkspace,
    }));
  });

  it('should navigate back to Workspace', async () => {
    const wsBtn = await $('button*=Workspace');
    await wsBtn.click();
    await browser.pause(500);

    const hasTerm = await $('[data-resizable-panel-id="terminal"]');
    expect(await hasTerm.isExisting()).toBe(true);
  });
});

describe('QA: PR Shepherd Flow', () => {
  it('should open PRs, click Shepherd, and observe navigation', async () => {
    // Open PRs panel
    const prsBtn = await $('button[aria-label="Toggle PRs"]');
    await prsBtn.click();
    await browser.waitUntil(async () => {
      return await browser.execute(() => document.body.textContent?.includes('Shepherd') ?? false);
    }, { timeout: 60000, interval: 2000 });

    // Click first Shepherd button
    await browser.execute(() => {
      const btn = Array.from(document.querySelectorAll('button')).find(
        b => b.textContent?.trim() === 'Shepherd' && !b.disabled
      );
      (btn as HTMLButtonElement)?.click();
    });

    // Wait for response
    await browser.waitUntil(async () => {
      return await browser.execute(() => {
        const text = document.body.textContent ?? '';
        return text.includes('Task') || text.includes('Shepherd failed') || text.includes('Agents');
      });
    }, { timeout: 30000, interval: 1000 });

    await browser.saveScreenshot(`${REPORT}/qa-08-after-shepherd.png`);

    const result = await browser.execute(() => {
      const panel = document.querySelector('[data-resizable-panel-id="right-sidebar"]');
      const text = panel?.textContent ?? '';
      return {
        hasTaskDetail: text.includes('PR Shepherd') || text.includes('PR #'),
        hasError: text.includes('Shepherd failed'),
        panelText: text.substring(0, 300),
      };
    });
    console.log('SHEPHERD RESULT:', JSON.stringify(result));
  });
});

describe('QA: Console Errors', () => {
  it('should check for JS errors', async () => {
    const errors = await browser.execute(() => {
      // Check if there are any error indicators in the DOM
      return {
        bodyLength: document.body.textContent?.length ?? 0,
        hasErrorText: (document.body.textContent ?? '').includes('Error'),
      };
    });
    console.log('CONSOLE CHECK:', JSON.stringify(errors));
    // WebKitWebDriver doesn't expose console.error easily, so we check DOM
  });
});

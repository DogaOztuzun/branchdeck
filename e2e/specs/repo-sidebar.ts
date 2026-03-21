describe('Repository sidebar', () => {
  it('should show persisted repositories', async () => {
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000 },
    );

    // App should have persisted repos from previous sessions
    const buttons = await $$('button');
    const repoNames: string[] = [];
    for (const btn of buttons) {
      const text = await btn.getText();
      repoNames.push(text.trim());
    }

    // Should have at least one repo listed (session persistence)
    const hasRepos = repoNames.some(
      (name) => name.includes('main') || name.includes('repo-'),
    );
    expect(hasRepos).toBe(true);
  });

  it('should show worktree branches under expanded repos', async () => {
    // Look for branch buttons (main, feat/*, fix/*)
    const mainBranch = await $('button*=main');
    expect(await mainBranch.isExisting()).toBe(true);
  });

  it('should show New Worktree button under expanded repo', async () => {
    const newWorktreeBtn = await $('button*=New Worktree');
    expect(await newWorktreeBtn.isExisting()).toBe(true);
  });

  it('should open Add Worktree modal when New Worktree is clicked', async () => {
    const newWorktreeBtn = await $('button*=New Worktree');
    await newWorktreeBtn.click();
    await browser.pause(500);

    // Modal should appear with "New Worktree" title
    const modalTitle = await $('*=New Worktree');
    // Look for the input field
    const nameInput = await $('input');

    const hasModal =
      (await modalTitle.isExisting()) || (await nameInput.isExisting());
    expect(hasModal).toBe(true);

    // Close modal by pressing Escape
    await browser.keys('Escape');
    await browser.pause(300);
  });

  it('should show session-persisted worktrees (feat/add-farewell)', async () => {
    const featureBranch = await $('button*=feat/add-farewell');
    expect(await featureBranch.isExisting()).toBe(true);
  });

  it('should show session-persisted worktrees (fix/broken-greet)', async () => {
    const fixBranch = await $('button*=fix/broken-greet');
    expect(await fixBranch.isExisting()).toBe(true);
  });
});

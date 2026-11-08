// Regression assets of findings that the generic L2 contract does not measure (ui-iteration-loop § 5).
// One test per finding; each failed before its fix.

import { test, expect, type Browser, type Page } from '@playwright/test';
import { base, hermetic } from './env.ts';

async function open(browser: Browser, path: string, width: number, height: number, theme = 'light'): Promise<Page> {
  const context = await browser.newContext({ viewport: { width, height }, colorScheme: theme as 'light' | 'dark', reducedMotion: 'reduce' });
  await hermetic(context);
  const page = await context.newPage();
  const resp = await page.goto(base() + path, { waitUntil: 'load' });
  expect(resp?.status(), `${path} is served`).toBe(200);
  await page.waitForTimeout(500);
  return page;
}

const gating = [
  { width: 1366, height: 768 },
  { width: 1280, height: 720 },
];

// UXF-012: on a phone or a tablet the menu button is not covered by the search box.
for (const vp of [{ width: 390, height: 844 }, { width: 768, height: 1024 }]) {
  test(`UXF-012 the navbar toggle is uncovered at ${vp.width}x${vp.height}`, async ({ browser }) => {
    const page = await open(browser, '/codespec/docs/getting-started', vp.width, vp.height);
    const hit = await page.evaluate(() => {
      const t = document.querySelector('.navbar__toggle') as HTMLElement | null;
      if (!t) return 'no toggle';
      const r = t.getBoundingClientRect();
      if (r.width < 24 || r.height < 24) return `toggle ${r.width}x${r.height}`;
      const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return e && t.contains(e) ? 'ok' : `covered by ${e?.tagName.toLowerCase()}.${(e as HTMLElement | null)?.className}`;
    });
    expect(hit).toBe('ok');
    await page.context().close();
  });
}

// UXF-007: the home page's primary action is in the first screen at both gating viewports.
for (const vp of gating) {
  test(`UXF-007 the home primary action is in view at ${vp.width}x${vp.height}`, async ({ browser }) => {
    const page = await open(browser, '/codespec/', vp.width, vp.height);
    const box = await page.locator('main').getByRole('link', { name: 'Get started' }).first().boundingBox();
    expect(box, 'a "Get started" link in main').not.toBeNull();
    expect(box!.y + box!.height, 'bottom edge inside the first screen').toBeLessThanOrEqual(vp.height);
    await page.context().close();
  });
}

// UXF-007: the home page does not repeat the Status table.
test('UXF-007 the home page links Status instead of repeating its rows', async ({ browser }) => {
  const page = await open(browser, '/codespec/', 1366, 768);
  const words = await page.evaluate(() => (document.querySelector('main')?.textContent ?? '').split(/\s+/).filter(Boolean).length);
  expect(words, 'words in main on the home page (1475 before the fix)').toBeLessThanOrEqual(700);
  await page.context().close();
});

// UXF-010: the current page is visible in the sidebar, on every kind of docs page.
for (const path of ['/codespec/docs/reference/cli', '/codespec/docs/reference/support', '/codespec/docs/examples/netcheck', '/codespec/docs/getting-started']) {
  for (const vp of gating) {
    test(`UXF-010 the active sidebar item is in view on ${path} at ${vp.width}x${vp.height}`, async ({ browser }) => {
      const page = await open(browser, path, vp.width, vp.height);
      const state = await page.evaluate(() => {
        const a = document.querySelector('.theme-doc-sidebar-menu .menu__link--active:not(.menu__link--sublist)') as HTMLElement | null;
        if (!a) return 'no active item';
        const r = a.getBoundingClientRect();
        return r.top >= 0 && r.bottom <= window.innerHeight ? 'ok' : `active item at ${Math.round(r.top)}..${Math.round(r.bottom)}`;
      });
      expect(state).toBe('ok');
      await page.context().close();
    });
  }
}

// UXF-014: commands and the page outline are set at 14 px or more.
test('UXF-014 code blocks and the table of contents are at least 14 px', async ({ browser }) => {
  const page = await open(browser, '/codespec/docs/getting-started', 1366, 768);
  const sizes = await page.evaluate(() => {
    const px = (sel: string) => {
      const e = document.querySelector(sel);
      return e ? parseFloat(getComputedStyle(e).fontSize) : -1;
    };
    return { code: px('.markdown pre code'), toc: px('.table-of-contents__link') };
  });
  expect(sizes.code, 'code block font size').toBeGreaterThanOrEqual(14);
  expect(sizes.toc, 'table of contents font size').toBeGreaterThanOrEqual(14);
  await page.context().close();
});

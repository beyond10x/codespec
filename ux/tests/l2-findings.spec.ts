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

// UXF-015: the cir/1 reference links Interface and TypeParam from its first screen.
for (const vp of gating) {
  test(`UXF-015 the cir/1 reference links Interface and TypeParam in its first screen at ${vp.width}x${vp.height}`, async ({ browser }) => {
    const page = await open(browser, '/codespec/docs/reference/cir-1', vp.width, vp.height);
    const where = await page.evaluate(() =>
      ['interface', 'typeparam'].map((id) => {
        const inView = [...document.querySelectorAll(`main a[href$="#${id}"]`)].some((a) => {
          const r = a.getBoundingClientRect();
          return r.width > 0 && r.top >= 0 && r.bottom <= window.innerHeight;
        });
        return `${id}: ${inView ? 'in view' : 'not in view'}`;
      }),
    );
    expect(where).toEqual(['interface: in view', 'typeparam: in view']);
    await page.context().close();
  });
}

// UXF-020: the sidebar's group names, the reader's main choice since only one group is open, are 14 px or more.
test('UXF-020 sidebar group names are at least 14 px', async ({ browser }) => {
  const page = await open(browser, '/codespec/docs/guides/add-an-adapter', 1366, 768);
  const sizes = await page.evaluate(() =>
    [...document.querySelectorAll('.theme-doc-sidebar-item-category-level-1 > .menu__list-item-collapsible > .menu__link')].map(
      (e) => `${(e.textContent ?? '').trim()} ${parseFloat(getComputedStyle(e).fontSize)}`,
    ),
  );
  expect(sizes.length, 'sidebar groups').toBeGreaterThan(0);
  expect(sizes.filter((s) => Number(s.split(' ').pop()) < 14), 'groups under 14 px').toEqual([]);
  await page.context().close();
});

// UXF-024: in reference tables the field names, types and commands (inline code) and the column
// headers are 14 px or more.
test('UXF-024 table code and table headers are at least 14 px', async ({ browser }) => {
  const page = await open(browser, '/codespec/docs/reference/cir-1', 1366, 768);
  const sizes = await page.evaluate(() => {
    const px = (sel: string) => {
      const e = document.querySelector(sel);
      return e ? parseFloat(getComputedStyle(e).fontSize) : -1;
    };
    return { 'td code': px('.markdown td code'), th: px('.markdown th') };
  });
  expect(sizes['td code'], 'inline code in a table cell').toBeGreaterThanOrEqual(14);
  expect(sizes.th, 'table header').toBeGreaterThanOrEqual(14);
  await page.context().close();
});

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

// UXF-029: at 14 px, code in table cells breaks where it must, so no Markdown table scrolls
// sideways at a gating viewport on the pages whose tables hold long identifiers.
for (const vp of gating) {
  test(`UXF-029 no Markdown table scrolls sideways at ${vp.width}x${vp.height}`, async ({ browser }) => {
    const wide: string[] = [];
    for (const p of ['concepts/pipeline', 'reference/cli', 'reference/crates', 'concepts/static-analysis', 'examples/netcheck']) {
      const page = await open(browser, `/codespec/docs/${p}`, vp.width, vp.height);
      const w = await page.evaluate(() =>
        [...document.querySelectorAll('.markdown .b10x-table-wrap')].filter((e) => e.scrollWidth > e.clientWidth + 1).map((e) => `${e.scrollWidth} in ${e.clientWidth}`),
      );
      wide.push(...w.map((x) => `${p}: ${x}`));
      await page.context().close();
    }
    expect(wide).toEqual([]);
  });
}

// UXF-030: on the support matrix and Status, the data a reader looks up (the feature id, its
// description and code, the Status group headers and chips) is 14 px or more.
test('UXF-030 support matrix features and Status headers and chips are at least 14 px', async ({ browser }) => {
  const small: string[] = [];
  for (const [p, sels] of [
    ['reference/support', ['a[class*="featureId"]', '[data-description]', '[data-description] code']],
    ['status', ['.b10x-status-table th', '.b10x-status-table .b10x-status']],
  ] as const) {
    const page = await open(browser, `/codespec/docs/${p}`, 1366, 768);
    const sizes = await page.evaluate(
      (ss) => ss.map((s) => [s, Math.min(...[...document.querySelectorAll(s)].map((e) => parseFloat(getComputedStyle(e).fontSize)))] as const),
      [...sels],
    );
    for (const [s, px] of sizes) if (!(px >= 14) || px === Infinity) small.push(`${p} ${s}: ${px}`);
    await page.context().close();
  }
  expect(small).toEqual([]);
});

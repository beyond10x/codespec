// L2 layout contract: every sitemap page × viewport × theme. Hard gates 1, 2 and 5 at the gating viewports;
// the measured-only viewports (round mode) are recorded and never fail.

import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { adapter } from '../adapter.ts';
import { CLS_INIT, checkAlwaysVisible, keyboardWalk, measureOverflow, readShift, readyTime } from '../core/layout.ts';
import { waiverFor } from '../core/findings.ts';
import { writeResult, type AxeRule, type Failure, type PageResult } from '../core/results.ts';
import { pageDrift } from '../core/scenarios.ts';
import { sitemapPaths } from '../core/site.ts';
import { FINDINGS, OUT, ROUND_MODE, SCENARIOS, SITE_ROOT, UX, base, hermetic } from './env.ts';

const clean = (s: string) => s.replace(/[|,]/g, ' ');

test('every sitemap page has a page scenario, and every page scenario is in the sitemap', () => {
  const drift = pageDrift(join(UX, adapter.dirs.scenarios), sitemapPaths(SITE_ROOT, adapter.site.sitemap));
  expect(drift, 'run `task ux-scenarios` to add page scenarios; remove stale ones by hand').toEqual({ missing: [], stale: [] });
});

const viewports = [
  ...adapter.viewports.gating.map((v) => ({ ...v, gating: true })),
  ...(ROUND_MODE ? adapter.viewports.measured.map((v) => ({ ...v, gating: false })) : []),
];

for (const sc of SCENARIOS.filter((s) => s.page)) {
  const path = sc.page!.path;
  for (const vp of viewports) {
    for (const theme of adapter.themes) {
      test(`L2 ${sc.name} ${vp.name} ${theme}`, async ({ browser }) => {
        const context = await browser.newContext({
          viewport: { width: vp.width, height: vp.height },
          colorScheme: theme as 'light' | 'dark',
          reducedMotion: 'reduce',
        });
        await hermetic(context);
        await context.addInitScript(CLS_INIT);
        const page = await context.newPage();
        const url = base() + path;
        const resp = await page.goto(url, { waitUntil: 'load' });
        expect(resp?.status(), `${path} is served`).toBe(200);
        const ready = await readyTime(page);
        await page.waitForTimeout(adapter.settleMs);
        const themeApplied = (await page.evaluate((a) => document.documentElement.getAttribute(a), adapter.themeAttribute)) === theme;
        expect(themeApplied, `${adapter.themeAttribute}=${theme} applied`).toBe(true);

        const shift = await readShift(page, ready);
        const overflow = await measureOverflow(page, adapter.scrollContainersAllowed);
        const top = await checkAlwaysVisible(page, adapter.alwaysVisible, path);
        const axe = await new AxeBuilder({ page }).withTags(adapter.axeTags).analyze();
        const rules: AxeRule[] = axe.violations.map((v) => ({
          id: v.id,
          impact: v.impact ?? 'unknown',
          nodes: v.nodes.length,
          help: v.help,
          targets: v.nodes.slice(0, 5).map((n) => n.target.join(' ')),
        }));
        const counts: Record<string, number> = {};
        for (const r of rules) counts[r.impact] = (counts[r.impact] ?? 0) + r.nodes;

        await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
        await page.waitForTimeout(300);
        const bottom = await checkAlwaysVisible(page, adapter.alwaysVisible, path);
        await page.evaluate(() => window.scrollTo(0, 0));

        let shot: string | undefined;
        const k = (gate: number, detail: string) => `gate${gate}|${path}|${vp.name}|${theme}|${clean(detail)}`;
        const failures: Failure[] = [];
        const fail = (gate: Failure['gate'], detail: string) => {
          const key = k(gate, detail);
          if (!failures.some((f) => f.key === key)) failures.push({ gate, key, detail, waivedBy: waiverFor(FINDINGS, key) });
        };
        if (overflow.docScroll > 0) fail(1, 'page-scroll');
        for (const o of overflow.overflow) fail(1, `overflow:${o.desc}`);
        for (const [state, list] of [['top', top], ['bottom', bottom]] as const)
          for (const v of list) if (v.status !== 'ok' && v.status !== 'n/a') fail(2, `${v.name}:${v.status}${v.status === 'missing' ? '' : '@' + state}`);
        for (const r of rules) if (r.impact === 'serious' || r.impact === 'critical') fail(5, `axe:${r.id}`);

        let keyboard = null;
        if (vp.gating) {
          const targets = adapter.keyboard.mustReach.map((n) => {
            const a = adapter.alwaysVisible.find((x) => x.name === n);
            if (!a) throw new Error(`keyboard target ${n} is not an always-visible element`);
            return { name: n, locate: a.locate };
          });
          keyboard = await keyboardWalk(page, url, adapter.keyboard.skipLink, targets, adapter.keyboard.maxTabs);
          if (!keyboard.skipLinkFirst) fail(5, 'keyboard:skip-link-first');
          else if (!keyboard.skipMovesFocus) fail(5, 'keyboard:skip-link-moves-focus');
          for (const [n, ok] of Object.entries(keyboard.reached)) if (!ok) fail(5, `keyboard:tab-reaches-${n}`);
        }

        if (ROUND_MODE && failures.length) {
          await page.goto(url, { waitUntil: 'load' });
          const sel = rules.filter((r) => r.impact === 'serious' || r.impact === 'critical').flatMap((r) => r.targets);
          await page.addStyleTag({ content: sel.map((s) => `${s}{outline:3px solid #e0007a !important;outline-offset:1px}`).join('\n') }).catch(() => {});
          await page.waitForTimeout(adapter.settleMs);
          const dir = join(OUT, 'shots-all');
          mkdirSync(dir, { recursive: true });
          shot = join(dir, `fail-${sc.name}-${vp.name}-${theme}.jpg`);
          await page.screenshot({ path: shot, type: 'jpeg', quality: 70, scale: 'css' });
        }

        const result: PageResult = {
          kind: 'page',
          scenario: sc.name,
          path,
          viewport: vp.name,
          theme,
          gating: vp.gating,
          themeApplied,
          overflow,
          visible: { top, bottom },
          axe: { counts, rules },
          keyboard,
          shift,
          failures,
          shot,
        };
        writeResult(OUT, result);
        await context.close();

        for (const f of failures.filter((f) => f.waivedBy)) test.info().annotations.push({ type: 'waived', description: `${f.key} by ${f.waivedBy}` });
        if (vp.gating) expect(failures.filter((f) => !f.waivedBy).map((f) => f.key), 'hard gate failures not waived by an open finding').toEqual([]);
      });
    }
  }
}

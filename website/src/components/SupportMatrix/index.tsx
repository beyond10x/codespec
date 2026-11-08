import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import clsx from 'clsx';
import {useHistory, useLocation} from '@docusaurus/router';
import styles from './styles.module.css';
import {
  CASES_URL,
  DEFAULT_VIEW,
  ROLES,
  STATES,
  STATE_CLASS,
  STATE_META,
  cellOf,
  countStates,
  familiesOf,
  familyLabel,
  matches,
  searchWithView,
  viewFromSearch,
  type Cell,
  type Feature,
  type Language,
  type Role,
  type SupportMatrixData,
  type View,
} from './model';

export type {SupportMatrixData} from './model';

/** Text with `code` spans, as the feature vocabulary writes its descriptions. */
function Inline({text}: {text: string}): React.ReactElement {
  const parts = text.split('`');
  return (
    <>
      {parts.map((part, i) => (i % 2 === 1 ? <code key={i}>{part}</code> : <React.Fragment key={i}>{part}</React.Fragment>))}
    </>
  );
}

function plain(text: string): string {
  return text.replace(/`/g, '');
}

interface Column {
  language: Language;
  role: Role;
  key: string;
}

interface Tip {
  feature: Feature;
  language: string;
  role: Role;
  cell: Cell;
  x: number;
  y: number;
  above: boolean;
}

/** The view in the URL query, read after hydration so the static HTML stays the default view. */
function useUrlView(data: SupportMatrixData): [View, (next: Partial<View>) => void] {
  const location = useLocation();
  const history = useHistory();
  const [view, setView] = useState<View>(DEFAULT_VIEW);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    setView(viewFromSearch(location.search, data));
    setReady(true);
    // Only the first location: later changes come from this component itself.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!ready) return;
    const search = searchWithView(window.location.search, view);
    if (search !== window.location.search) {
      history.replace({pathname: window.location.pathname, search, hash: window.location.hash});
    }
  }, [view, ready, history]);

  const update = useCallback((next: Partial<View>) => setView((current) => ({...current, ...next})), []);

  return [view, update];
}

function Chips({
  label,
  options,
  value,
  onChange,
}: {
  label: string;
  options: ReadonlyArray<{value: string; label: string; count?: number}>;
  value: string;
  onChange: (value: string) => void;
}): React.ReactElement {
  return (
    <div className={styles.chips} role="group" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          className={clsx(styles.chip, value === o.value && styles.chipActive)}
          aria-pressed={value === o.value}
          onClick={() => onChange(o.value)}>
          {o.label}
          {o.count !== undefined ? <span className={styles.chipCount}>{o.count}</span> : null}
        </button>
      ))}
    </div>
  );
}

function Coverage({columns, features}: {columns: Column[]; features: ReadonlyArray<Feature>}): React.ReactElement {
  const shown = features.length;
  return (
    <div className={styles.coverage} aria-label="Coverage per language and role" role="list">
      {columns.map(({language, role, key}) => {
        const c = countStates(language[role], features);
        // Features not applicable to this role (facts for a reader) are out of the denominator.
        const total = shown - c['n/a'];
        const absent = c.none === total && total > 0;
        const inapplicable = total === 0 && c['n/a'] > 0;
        const empty = absent ? `no ${role === 'read' ? 'reader' : 'writer'}` : inapplicable ? 'not applicable to this role' : '';
        const pct = (n: number) => (total ? (100 * n) / total : 0);
        const share = Math.round(pct(c.verified + c.claimed));
        const summary = empty
          ? `${language.name} ${role}: ${empty}`
          : `${language.name} ${role}: ${c.verified} verified and ${c.claimed} claimed of ${total} applicable features, ${share}%`;
        return (
          <div key={key} className={styles.bar} role="listitem" data-coverage={key} aria-label={summary}>
            <span className={styles.barName} aria-hidden="true">
              <strong>{language.name}</strong> <span className={styles.barRole}>{role}</span>
            </span>
            <span className={styles.track} aria-hidden="true">
              <span className={clsx(styles.seg, styles.segVerified)} style={{width: `${pct(c.verified)}%`}} />
              <span className={clsx(styles.seg, styles.segClaimed)} style={{width: `${pct(c.claimed)}%`}} />
            </span>
            <span className={styles.barNums} aria-hidden="true">
              {empty ? (
                <span className={styles.barNone}>{empty}</span>
              ) : (
                <>
                  <strong>{share}%</strong>{' '}
                  <span>
                    {c.verified} ✓ · {c.claimed} ○ / {total}
                  </span>
                </>
              )}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function Legend({columns, languages, features}: {columns: Column[]; languages: ReadonlyArray<Language>; features: ReadonlyArray<Feature>}): React.ReactElement {
  const counts = columns.map(({language, role}) => countStates(language[role], features));
  return (
    <div className={clsx(styles.scroll, styles.legend)} tabIndex={0} role="region" aria-label="Legend: cells per state and column">
      <table className={styles.table}>
        <caption className={styles.caption}>
          Legend: cells per state and column, over the {features.length} features shown
        </caption>
        <thead>
          <tr>
            <th scope="col" rowSpan={2} className={clsx(styles.stick, styles.corner)}>
              State
            </th>
            {languages.map((language) => (
              <th key={language.name} scope="colgroup" colSpan={2} className={styles.language}>
                {language.name}
              </th>
            ))}
          </tr>
          <tr>
            {columns.map(({key, role}) => (
              <th key={key} scope="col" className={clsx(styles.role, role === 'read' && styles.pairStart)}>
                {role}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {STATES.map((state) => (
            <tr key={state}>
              <th scope="row" className={styles.stick}>
                <div className={styles.stateName}>
                  <span aria-hidden="true" className={clsx(styles.swatch, styles[STATE_CLASS[state]])}>
                    {STATE_META[state].symbol}
                  </span>
                  <span>
                    <strong>{STATE_META[state].label}</strong>
                    <span className={styles.meaning}>{STATE_META[state].meaning}</span>
                  </span>
                </div>
              </th>
              {columns.map(({key, role}, i) => (
                <td key={key} className={clsx(styles.count, role === 'read' && styles.pairStart, counts[i][state] === 0 && styles.zero)}>
                  {counts[i][state]}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function cellLabel(feature: Feature, language: string, role: Role, cell: Cell): string {
  const meta = STATE_META[cell.state];
  const state = cell.state === 'n/a' ? `${meta.label}, ${meta.meaning}` : meta.label;
  const base = `${feature.id}, ${language} ${role}: ${state}`;
  return cell.note ? `${base}. ${cell.note}` : base;
}

export default function SupportMatrix({data}: {data: SupportMatrixData}): React.ReactElement {
  const [view, update] = useUrlView(data);
  const families = useMemo(() => familiesOf(data), [data]);
  const tableRef = useRef<HTMLTableElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState<[number, number]>([0, 1]);
  const [tip, setTip] = useState<Tip | null>(null);

  const groups = useMemo(
    () =>
      families
        .filter((f) => view.family === 'all' || f === view.family)
        .map((f) => ({family: f, features: data.features.filter((x) => x.family === f && matches(x, view.query))}))
        .filter((g) => g.features.length > 0),
    [data, families, view.family, view.query],
  );
  const shown = useMemo(() => groups.flatMap((g) => g.features), [groups]);

  const languages = useMemo(
    () => (view.lang === 'all' ? data.languages : data.languages.filter((l) => l.name === view.lang)),
    [data, view.lang],
  );
  const columns: Column[] = languages.flatMap((language) =>
    ROLES.map((role) => ({language, role, key: `${language.name}-${role}`})),
  );

  const lastRow = Math.max(shown.length - 1, 0);
  const lastCol = columns.length;
  const [ar, ac] = [Math.min(active[0], lastRow), Math.min(active[1], lastCol)];

  const anchor = useRef<HTMLElement | null>(null);
  const hideTip = useCallback(() => {
    anchor.current = null;
    setTip(null);
  }, []);

  /** Where the detail sits for `el`: below it, or above it near the bottom of the window. */
  const place = (el: HTMLElement): Pick<Tip, 'x' | 'y' | 'above'> => {
    const rect = el.getBoundingClientRect();
    const above = rect.bottom + 160 > window.innerHeight;
    return {
      x: Math.min(Math.max(rect.left + rect.width / 2, 150), window.innerWidth - 150),
      y: above ? rect.top - 6 : rect.bottom + 6,
      above,
    };
  };

  // On scroll the detail follows its cell, and goes away once the cell leaves the visible matrix.
  useEffect(() => {
    const scroller = scrollRef.current;
    const follow = () => {
      const el = anchor.current;
      if (!el || !scroller) return;
      const r = el.getBoundingClientRect();
      const s = scroller.getBoundingClientRect();
      const visible = r.bottom > s.top && r.top < s.bottom && r.right > s.left && r.left < s.right;
      if (!visible) {
        hideTip();
        return;
      }
      setTip((t) => (t ? {...t, ...place(el)} : t));
    };
    window.addEventListener('scroll', follow, {passive: true});
    scroller?.addEventListener('scroll', follow, {passive: true});
    return () => {
      window.removeEventListener('scroll', follow);
      scroller?.removeEventListener('scroll', follow);
    };
  }, [hideTip]);

  const showTip = (el: HTMLElement, feature: Feature, language: string, role: Role, cell: Cell) => {
    anchor.current = el;
    setTip({feature, language, role, cell, ...place(el)});
  };

  const focusCell = (r: number, c: number) => {
    const row = Math.min(Math.max(r, 0), lastRow);
    const col = Math.min(Math.max(c, 0), lastCol);
    setActive([row, col]);
    const el = tableRef.current?.querySelector<HTMLElement>(`[data-cell="${row}:${col}"]`);
    el?.focus();
  };

  const onGridKey = (event: React.KeyboardEvent<HTMLTableElement>) => {
    const target = event.target as HTMLElement;
    const at = target.dataset.cell;
    if (!at) return;
    const [r, c] = at.split(':').map(Number);
    const moves: Record<string, [number, number]> = {
      ArrowRight: [r, c + 1],
      ArrowLeft: [r, c - 1],
      ArrowDown: [r + 1, c],
      ArrowUp: [r - 1, c],
      PageDown: [r + 10, c],
      PageUp: [r - 10, c],
      Home: event.ctrlKey ? [0, 0] : [r, 0],
      End: event.ctrlKey ? [lastRow, lastCol] : [r, lastCol],
    };
    if (event.key === 'Escape') {
      setTip(null);
      return;
    }
    const to = moves[event.key];
    if (!to) return;
    event.preventDefault();
    focusCell(to[0], to[1]);
  };

  let rowIndex = -1;

  return (
    <div className={styles.root} data-support-matrix="">
      <div className={styles.controls}>
        <label className={styles.search}>
          <span className={styles.searchLabel}>Search feature ids</span>
          <input
            type="search"
            value={view.query}
            placeholder="e.g. sem. or for or +="
            onChange={(e) => update({query: e.target.value})}
            aria-controls="support-matrix-table"
          />
        </label>
        <Chips
          label="Filter by family"
          value={view.family}
          onChange={(family) => update({family})}
          options={[
            {value: 'all', label: 'All', count: data.features.length},
            ...families.map((f) => ({value: f, label: familyLabel(f), count: data.features.filter((x) => x.family === f).length})),
          ]}
        />
        <Chips
          label="Language"
          value={view.lang}
          onChange={(lang) => update({lang})}
          options={[{value: 'all', label: 'All languages'}, ...data.languages.map((l) => ({value: l.name, label: l.name}))]}
        />
      </div>

      <Coverage columns={columns} features={shown} />
      <Legend columns={columns} languages={languages} features={shown} />

      <p className={styles.status} role="status">
        {shown.length === data.features.length
          ? `${shown.length} features`
          : `${shown.length} of ${data.features.length} features`}
        {view.lang !== 'all' ? `, ${view.lang} only` : ''}. Arrow keys move between cells.
      </p>

      <div ref={scrollRef} className={clsx(styles.scroll, styles.matrixScroll)} data-matrix-scroll="">
        <table
          id="support-matrix-table"
          ref={tableRef}
          className={clsx(styles.table, styles.matrix)}
          role="grid"
          aria-label="Support matrix"
          aria-rowcount={shown.length + 2}
          onKeyDown={onGridKey}>
          <thead>
            <tr>
              <th scope="col" rowSpan={2} className={clsx(styles.stick, styles.corner, styles.head)}>
                Feature
              </th>
              {languages.map((language) => (
                <th key={language.name} scope="colgroup" colSpan={2} className={clsx(styles.head, styles.language)}>
                  {language.name}
                </th>
              ))}
            </tr>
            <tr>
              {columns.map(({key, role}) => (
                <th key={key} scope="col" className={clsx(styles.head, styles.headSecond, styles.role, role === 'read' && styles.pairStart)}>
                  {role}
                </th>
              ))}
            </tr>
          </thead>
          {groups.map((group) => (
            <tbody key={group.family}>
              <tr className={styles.familyRow}>
                <th scope="colgroup" colSpan={1 + columns.length} className={styles.familyHead}>
                  <span className={styles.stickLabel}>
                    {familyLabel(group.family)} <span className={styles.familyCount}>{group.features.length}</span>
                  </span>
                </th>
              </tr>
              {group.features.map((feature) => {
                rowIndex += 1;
                const r = rowIndex;
                const tab = (c: number) => (r === ar && c === ac ? 0 : -1);
                const onFocus = (c: number) => () => setActive([r, c]);
                return (
                  <tr key={feature.id} data-feature-row={feature.id} data-family={feature.family}>
                    <th scope="row" className={clsx(styles.stick, styles.feature)}>
                      <span className={styles.featureLine}>
                        {feature.case ? (
                          <a
                            href={`${CASES_URL}${feature.case}`}
                            className={styles.featureId}
                            data-cell={`${r}:0`}
                            tabIndex={tab(0)}
                            onFocus={onFocus(0)}
                            aria-label={`${feature.id}: conformance case ${feature.case}`}>
                            {feature.id}
                          </a>
                        ) : (
                          <span
                            className={clsx(styles.featureId, styles.noCase)}
                            data-cell={`${r}:0`}
                            tabIndex={tab(0)}
                            onFocus={onFocus(0)}
                            aria-label={`${feature.id}: no conformance case yet`}
                            title="No conformance case yet">
                            {feature.id}
                          </span>
                        )}
                        {feature.op ? <code className={styles.op}>{feature.op}</code> : null}
                      </span>
                      {feature.description ? (
                        <span className={styles.description} data-description="" title={plain(feature.description)}>
                          <Inline text={feature.description} />
                        </span>
                      ) : null}
                    </th>
                    {columns.map(({key, language, role}, i) => {
                      const cell = cellOf(language[role], feature.id);
                      const c = i + 1;
                      return (
                        <td
                          key={key}
                          className={clsx(styles.cell, styles[STATE_CLASS[cell.state]], role === 'read' && styles.pairStart, cell.note && styles.hasNote)}
                          data-cell={`${r}:${c}`}
                          data-state={cell.state}
                          data-lang={language.name}
                          data-role={role}
                          tabIndex={tab(c)}
                          aria-label={cellLabel(feature, language.name, role, cell)}
                          onFocus={(e) => {
                            setActive([r, c]);
                            showTip(e.currentTarget, feature, language.name, role, cell);
                          }}
                          onBlur={hideTip}
                          onMouseEnter={(e) => showTip(e.currentTarget, feature, language.name, role, cell)}
                          onMouseLeave={hideTip}>
                          <span aria-hidden="true" className={styles.symbol}>
                            {STATE_META[cell.state].symbol}
                          </span>
                        </td>
                      );
                    })}
                  </tr>
                );
              })}
            </tbody>
          ))}
        </table>
        {shown.length === 0 ? <p className={styles.empty}>No feature id matches “{view.query}”.</p> : null}
      </div>

      {tip ? (
        <div
          role="tooltip"
          className={clsx(styles.tip, tip.above && styles.tipAbove)}
          style={{left: tip.x, top: tip.y}}>
          <div className={styles.tipHead}>
            <span className={clsx(styles.swatch, styles[STATE_CLASS[tip.cell.state]])} aria-hidden="true">
              {STATE_META[tip.cell.state].symbol}
            </span>
            <span>
              <strong>{STATE_META[tip.cell.state].label}</strong>
              <span className={styles.tipMeaning}>{STATE_META[tip.cell.state].meaning}</span>
            </span>
          </div>
          <div className={styles.tipBody}>
            <code>{tip.feature.id}</code> · {tip.language} {tip.role}
          </div>
          {tip.feature.description ? (
            <div className={styles.tipDesc}>
              <Inline text={tip.feature.description} />
            </div>
          ) : null}
          {tip.cell.note ? <div className={styles.tipNote}>{tip.cell.note}</div> : null}
        </div>
      ) : null}
    </div>
  );
}

/** The `codespec-support-matrix/1` document and what the component derives from it. */

/** The five states of a cell, in legend order (`schemas/support-matrix-1.schema.json`). */
export const STATES = ['verified', 'claimed', 'failing', 'refused', 'n/a', 'none'] as const;
export type State = (typeof STATES)[number];

/** The feature families, in display order. */
export const FAMILIES = [
  'statement',
  'expression',
  'operator',
  'intrinsic',
  'type',
  'fact',
  'semantic',
] as const;

export const FAMILY_LABEL: Record<string, string> = {
  statement: 'Statements',
  expression: 'Expressions',
  operator: 'Operators',
  intrinsic: 'Intrinsics',
  type: 'Types',
  fact: 'Facts',
  semantic: 'Semantics',
};

export const STATE_META: Record<State, {symbol: string; label: string; meaning: string}> = {
  verified: {symbol: '✓', label: 'verified', meaning: 'claimed, conformance passes'},
  claimed: {symbol: '○', label: 'claimed', meaning: 'in the manifest, no report or case yet'},
  failing: {symbol: '✗', label: 'failing', meaning: 'claimed, conformance fails'},
  refused: {symbol: '⊘', label: 'refused', meaning: 'not claimed'},
  'n/a': {symbol: 'n/a', label: 'n/a', meaning: 'not applicable to this role'},
  none: {symbol: '–', label: 'none', meaning: 'no adapter for that role'},
};

/** The CSS module class of a state; `n/a` is no valid class name. */
export const STATE_CLASS: Record<State, string> = {
  verified: 'verified',
  claimed: 'claimed',
  failing: 'failing',
  refused: 'refused',
  'n/a': 'na',
  none: 'none',
};

export const ROLES = ['read', 'write'] as const;
export type Role = (typeof ROLES)[number];

export const CASES_URL = 'https://github.com/beyond10x/codespec/tree/main/conformance/cases/';

/** A cell as the document holds it: a state, or a state with a note. */
export type RawCell = string | {state: string; note?: string};

/** A `codespec-support-matrix/1` document as the JSON import types it. */
export interface SupportMatrixData {
  format: string;
  cir?: string;
  features: ReadonlyArray<{
    id: string;
    family: string;
    op?: string;
    description?: string;
    case?: string | null;
  }>;
  languages: ReadonlyArray<{
    name: string;
    read: Readonly<Record<string, RawCell>>;
    write: Readonly<Record<string, RawCell>>;
  }>;
}

export type Feature = SupportMatrixData['features'][number];
export type Language = SupportMatrixData['languages'][number];

export interface Cell {
  state: State;
  note?: string;
}

function isState(value: unknown): value is State {
  return typeof value === 'string' && (STATES as ReadonlyArray<string>).includes(value);
}

/** A cell's state and note; a missing or unknown value reads as `none`, never as support. */
export function cellOf(cells: Readonly<Record<string, RawCell>>, id: string): Cell {
  const raw = cells[id];
  if (typeof raw === 'string') {
    return {state: isState(raw) ? raw : 'none'};
  }
  if (raw && typeof raw === 'object') {
    return {state: isState(raw.state) ? raw.state : 'none', note: raw.note || undefined};
  }
  return {state: 'none'};
}

export function familyLabel(family: string): string {
  return FAMILY_LABEL[family] ?? family;
}

/** Families present in the document: the known ones in display order, then any others sorted. */
export function familiesOf(data: SupportMatrixData): string[] {
  const present = new Set(data.features.map((f) => f.family));
  const known: string[] = FAMILIES.filter((f) => present.has(f));
  const other = [...present]
    .filter((f) => !(FAMILIES as ReadonlyArray<string>).includes(f))
    .sort();
  return [...known, ...other];
}

/** Whether `feature` matches the search text: a case-insensitive substring of its id or operator. */
export function matches(feature: Feature, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return feature.id.toLowerCase().includes(q) || (feature.op ?? '').toLowerCase().includes(q);
}

export type Counts = Record<State, number>;

export function countStates(
  cells: Readonly<Record<string, RawCell>>,
  features: ReadonlyArray<Feature>,
): Counts {
  const count: Counts = {verified: 0, claimed: 0, failing: 0, refused: 0, 'n/a': 0, none: 0};
  for (const feature of features) {
    count[cellOf(cells, feature.id).state] += 1;
  }
  return count;
}

/** The view a URL query names: `family`, `q` and `lang`; anything unknown falls back to all. */
export interface View {
  family: string;
  query: string;
  lang: string;
}

export const DEFAULT_VIEW: View = {family: 'all', query: '', lang: 'all'};

export function viewFromSearch(search: string, data: SupportMatrixData): View {
  const params = new URLSearchParams(search);
  const family = params.get('family') ?? 'all';
  const lang = params.get('lang') ?? 'all';
  const langMatch = data.languages.find((l) => l.name.toLowerCase() === lang.toLowerCase());
  return {
    family: familiesOf(data).includes(family) ? family : 'all',
    query: params.get('q') ?? '',
    lang: langMatch ? langMatch.name : 'all',
  };
}

/** `search` with the view's parameters set, defaults removed and every other parameter kept. */
export function searchWithView(search: string, view: View): string {
  const params = new URLSearchParams(search);
  const set = (key: string, value: string, fallback: string) => {
    if (value && value !== fallback) params.set(key, value);
    else params.delete(key);
  };
  set('family', view.family, 'all');
  set('q', view.query, '');
  set('lang', view.lang, 'all');
  const out = params.toString();
  return out ? `?${out}` : '';
}

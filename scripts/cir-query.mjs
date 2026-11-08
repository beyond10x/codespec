// cir-query: bug and performance queries over a directory of cir/1 documents only (bodies, call lists, method sets, inits, facts); reads no source.
//
// usage: node scripts/cir-query.mjs <query> <docs dir> --path <import path prefix> [query options]
//   dead                     functions of the module no function, init or method set of the linked set reaches
//   direct-calls --pkgs a,b  call sites into the named packages, with the exported functions of the module that reach them
//   lock-spans               a Lock/RLock call list position, the next unlock of the same kind; function values and
//                            interface methods called between them; a lock with no unlock in the function
//   growth                   an insertion called after a receive in one function, every removal positioned before the
//                            first receive (cleanup registered up front): a collection that grows per received input
//   contract --table <json>  a call to `open` with none of `consume` in the same function (default table: bufio.Scanner.Scan, consumed by Err)
//   int-from-text            read bodies: an integer parsed from text returned or stored with no comparison on it
//   swallowed                read bodies: `if err != nil` whose branch returns a nil error
//   repeated                 the same callee called 3 or more times in one function (repeated lookup / recomputation)
//   unordered                the unordered_iteration fact of documents already passed by `codespec pass`
// Positions are the documents' own `file:line`; a function's calls are ordered by line.
import fs from 'node:fs';
import path from 'node:path';

const [query, docsDir, ...rest] = process.argv.slice(2);
const opt = (name, def) => { const i = rest.indexOf(name); return i >= 0 ? rest[i + 1] : def; };
const prefix = opt('--path', '');
if (!query || !docsDir || !prefix) { console.error('usage: node scripts/cir-query.mjs <query> <docs dir> --path <import path prefix> [query options]'); process.exit(2); }

const docs = fs.readdirSync(docsDir).filter((f) => f.endsWith('.cir.json')).sort()
  .map((f) => JSON.parse(fs.readFileSync(path.join(docsDir, f), 'utf8')));
const byPath = new Map(docs.map((d) => [d.path, d]));
const own = docs.filter((d) => d.path === prefix || d.path.startsWith(prefix + '/'));
const rel = (d) => (d.path === prefix ? '' : d.path.slice(prefix.length + 1) + '/');
const lineOf = (pos) => Number(String(pos).split(':')[1]);

function walk(n, f) {
  if (Array.isArray(n)) { for (const x of n) walk(x, f); return; }
  if (n && typeof n === 'object') { f(n); for (const v of Object.values(n)) walk(v, f); }
}
const recvName = (t) => (!t ? undefined : t.k === 'ptr' ? recvName(t.elem) : t.name);

// Every function of a document with its calls as call-list records (read bodies converted).
function functions(d) {
  const out = [];
  for (const fn of d.funcs || []) {
    const calls = [];
    let last = fn.pos;
    walk(fn.body, (n) => {
      if (n.pos) last = n.pos;
      if (n.k === 'call') calls.push({ k: 'func', name: n.name, pos: last });
      else if (n.k === 'extern') calls.push({ k: 'func', path: n.path, name: n.name, pos: last });
      else if (n.k === 'method' && n.name && n.recv && !n.methods && !n.sig) calls.push({ k: 'method', recv: n.recv, name: n.name, pos: last });
      else if (n.k === 'foreign') walk(n.calls, (m) => { if (m.pos && m.k && m.k !== 'lambda') calls.push(m); });
    });
    out.push({ d, name: fn.name, recv: recvName(fn.recv && fn.recv.t), pos: fn.pos, exported: fn.exported, body: fn.body, results: fn.results, calls });
  }
  for (const o of d.opaque || []) {
    const calls = [];
    walk(o.calls, (m) => { if (m.pos && m.k && m.k !== 'lambda') calls.push(m); });
    out.push({ d, name: o.name, recv: o.recv, pos: o.pos, exported: /^[A-Z]/.test(o.name), calls });
  }
  for (const fn of out) fn.calls.sort((a, b) => lineOf(a.pos) - lineOf(b.pos));
  return out;
}
const all = own.flatMap(functions);
const id = (fn) => `${fn.d.path}.${fn.recv ? fn.recv + '.' : ''}${fn.name}`;
const at = (fn, pos) => `${rel(fn.d)}${pos || fn.pos}`;
const callee = (c) => `${c.path ? c.path + '.' : ''}${c.recv ? c.recv + '.' : ''}${c.iface ? c.iface + '.' : ''}${c.name || '(function value)'}`;

// Call graph inside the module: a call by (path, recv, name); an interface call reaches every
// method of that name; a ref is a use.
const fnByKey = new Map();
for (const fn of all) fnByKey.set(`${fn.d.path}|${fn.recv || ''}|${fn.name}`, fn);
function targets(fn, c) {
  const p = c.path || fn.d.path;
  if (c.k === 'iface') return all.filter((g) => g.recv && g.name === c.name);
  const g = fnByKey.get(`${p}|${c.recv || ''}|${c.name}`);
  return g ? [g] : [];
}
const callers = new Map();
for (const fn of all) for (const c of fn.calls) for (const g of targets(fn, c)) {
  if (!callers.has(g)) callers.set(g, new Set());
  callers.get(g).add(fn);
}

if (query === 'dead') {
  const used = new Set();
  for (const d of own) walk(d.inits, (m) => { if (m.name) used.add(`${m.path || d.path}|${m.recv || ''}|${m.name}`); });
  // a method in the method set of a type is reachable through an interface conversion
  const ifaceNames = new Set();
  for (const d of docs) walk(d.opaque, (m) => { if (m.k === 'iface') for (const x of m.methods || []) ifaceNames.add(x.name); });
  for (const d of docs) for (const t of d.types || []) if (t.k === 'iface') for (const m of t.methods || []) ifaceNames.add(m.name);
  for (const fn of all) {
    if (fn.exported || fn.name === 'init' || fn.name === 'main') continue;
    if (fn.recv && ifaceNames.has(fn.name)) continue;
    if (callers.has(fn) && [...callers.get(fn)].some((g) => g !== fn)) continue;
    if (used.has(`${fn.d.path}|${fn.recv || ''}|${fn.name}`)) continue;
    // a ref anywhere in the module (a function passed as a value) counts as a use
    if (all.some((g) => g.calls.some((c) => c.k === 'ref' && c.name === fn.name))) continue;
    console.log(`${at(fn)}\t${id(fn)}\tno caller, ref or init use in the linked set`);
  }
} else if (query === 'direct-calls') {
  const pkgs = opt('--pkgs', 'math/rand').split(',');
  for (const fn of all) for (const c of fn.calls) {
    if (!c.path || !pkgs.includes(c.path)) continue;
    // exported functions of the module that reach fn
    const seen = new Set([fn]); const q = [fn]; const roots = [];
    while (q.length) { const g = q.shift(); if (g.exported && g !== fn) roots.push(id(g).replace(prefix, '')); for (const h of callers.get(g) || []) if (!seen.has(h)) { seen.add(h); q.push(h); } }
    console.log(`${at(fn, c.pos)}\t${id(fn)}\t${callee(c)}\treached from ${roots.length} exported: ${roots.slice(0, 8).join(' ')}`);
  }
} else if (query === 'callers') {
  // callers of every function whose id ends with --of (e.g. Parser.Next), with call positions
  const of = opt('--of', '');
  for (const fn of all) for (const c of fn.calls) for (const g of targets(fn, c)) if (id(g).endsWith(of)) console.log(`${at(fn, c.pos)}\t${id(fn)}\tcalls ${id(g)}`);
} else if (query === 'lock-spans') {
  // sync locks, and module methods named lock/rlock (wrappers whose body takes a lock)
  const isLock = (c) => (c.path === 'sync' && (c.name === 'Lock' || c.name === 'RLock')) || (!c.path && c.recv && (c.name === 'lock' || c.name === 'rlock'));
  const unlockOf = (c) => ({ RLock: 'RUnlock', Lock: 'Unlock', lock: 'unlock', rlock: 'runlock' })[c.name];
  for (const fn of all) for (let i = 0; i < fn.calls.length; i++) {
    const lk = fn.calls[i];
    if (!isLock(lk)) continue;
    const j = fn.calls.findIndex((c, k) => k > i && c.path === lk.path && c.recv === lk.recv && c.name === unlockOf(lk));
    if (j < 0) { console.log(`${at(fn, lk.pos)}\t${id(fn)}\t${callee(lk)}\tno ${unlockOf(lk)} in the function`); continue; }
    for (const c of fn.calls.slice(i + 1, j)) {
      if (c.k === 'value') console.log(`${at(fn, lk.pos)}\t${id(fn)}\t${callee(lk)}\tfunction value called at line ${lineOf(c.pos)} before ${unlockOf(lk)} at ${lineOf(fn.calls[j].pos)}`);
      if (c.k === 'iface' && !/^(io|fmt|net)\./.test(c.iface)) console.log(`${at(fn, lk.pos)}\t${id(fn)}\t${callee(lk)}\tinterface call ${c.iface}.${c.name} at line ${lineOf(c.pos)} before ${unlockOf(lk)} at ${lineOf(fn.calls[j].pos)}`);
    }
  }
} else if (query === 'growth') {
  const receive = /^(Read|ReadFrom|ReadMsgUDP|ReadFromUDP|Accept|NextFrame|Recv)$/;
  const insert = /^(Add|Put|Store|put|add|Set|Insert|Append)$/;
  const remove = /^(Delete|delete|Remove|remove|drop|Drop|Clear|Clean|Evict)\w*$/;
  for (const fn of all) {
    const rx = fn.calls.filter((c) => c.name && receive.test(c.name));
    if (!rx.length) continue;
    const first = lineOf(rx[0].pos);
    const ins = fn.calls.filter((c) => c.name && insert.test(c.name) && lineOf(c.pos) > first && c.recv);
    const rm = fn.calls.filter((c) => c.name && remove.test(c.name));
    for (const c of ins) {
      const sameRecv = rm.filter((r) => r.recv === c.recv);
      if (sameRecv.every((r) => lineOf(r.pos) < first)) console.log(`${at(fn, c.pos)}\t${id(fn)}\t${callee(c)} after receive ${callee(rx[0])} at ${first}; removals of ${c.recv}: ${sameRecv.map((r) => r.name + '@' + lineOf(r.pos)).join(', ') || 'none'} (all before the first receive)`);
    }
  }
} else if (query === 'contract') {
  const table = JSON.parse(opt('--table', JSON.stringify([
    { open: 'bufio.Scanner.Scan', consume: ['bufio.Scanner.Err'] },
  ])));
  for (const fn of all) for (const { open, consume } of table) {
    const o = fn.calls.filter((c) => callee(c) === open);
    if (!o.length) continue;
    const used = fn.calls.filter((c) => consume.includes(callee(c)));
    const other = fn.calls.filter((c) => lineOf(c.pos) > lineOf(o[0].pos) && /Read/.test(c.name || '') && !consume.includes(callee(c)));
    if (used.length < consume.length || other.length) console.log(`${at(fn, o[0].pos)}\t${id(fn)}\t${open}: consumed by ${used.map((c) => callee(c) + '@' + lineOf(c.pos)).join(', ') || 'nothing'}; missing ${consume.filter((x) => !used.some((c) => callee(c) === x)).join(', ') || '-'}; payload read elsewhere: ${other.map((c) => callee(c) + '@' + lineOf(c.pos)).join(', ') || '-'}`);
  }
} else if (query === 'int-from-text') {
  const parsers = /^strconv\.(Atoi|ParseInt|ParseUint)$/;
  for (const fn of all) {
    if (!fn.body) continue;
    walk(fn.body, (s) => {
      if (!(s.k === 'let' || s.k === 'assign') || !s.rhs || s.rhs.length !== 1) return;
      const r = s.rhs[0];
      if (r.k !== 'extern' || !parsers.test(`${r.path}.${r.name}`)) return;
      const name = s.names ? s.names[0] : s.lhs && s.lhs[0] && s.lhs[0].name;
      let compared = false;
      walk(fn.body, (b) => { if (b.k === 'binop' && /^(<|<=|>|>=)$/.test(b.op) && [b.x, b.y].some((e) => e && e.k === 'var' && e.name === name)) compared = true; });
      if (!compared) console.log(`${at(fn, s.pos)}\t${id(fn)}\t${r.path}.${r.name} into ${name}: never compared with a bound before it leaves the function`);
    });
  }
  // Opaque functions: a call to a text-to-integer function, recognised by its cir/1 signature in
  // the callee's document (first parameter string, results (integer, error)). The flow of the
  // result is not in cir/1; a signed result of full width means negatives and values beyond any
  // narrower domain (ports, codes) are accepted unless the function bounds them.
  const textToInt = (c) => {
    const d = c.path && byPath.get(c.path);
    const f = d && (d.funcs || []).find((x) => x.name === c.name && !x.recv);
    if (!f || !f.params || !f.params.length || f.params[0].t.k !== 'string') return undefined;
    const r = f.results || [];
    return r.length === 2 && r[0].k === 'int' && r[1].k === 'error' ? r[0] : undefined;
  };
  for (const fn of all) {
    if (fn.body) continue;
    const parses = fn.calls.filter((c) => c.k === 'func' && textToInt(c));
    for (const c of parses) {
      const t = textToInt(c);
      console.log(`${at(fn, c.pos)}\t${id(fn)}\t${callee(c)} gives ${t.signed ? 'signed' : 'unsigned'} ${t.width}-bit int; opaque caller, bounds not visible (${parses.length} parse call(s) here)`);
    }
  }
} else if (query === 'swallowed') {
  for (const fn of all) {
    if (!fn.body) continue;
    walk(fn.body, (s) => {
      if (s.k !== 'if' || !s.cond || s.cond.k !== 'binop' || s.cond.op !== '!=') return;
      const v = [s.cond.x, s.cond.y].find((e) => e && e.k === 'var' && e.t && e.t.k === 'error');
      if (!v) return;
      walk(s.then, (r) => {
        if (r.k !== 'return' || !r.results || !r.results.length) return;
        const e = r.results[r.results.length - 1];
        if (e.k === 'nil' || (e.k === 'lit' && e.value === 'nil')) console.log(`${at(fn, r.pos)}\t${id(fn)}\t${v.name} != nil, then returns a nil error`);
      });
    });
  }
} else if (query === 'repeated') {
  for (const fn of all) {
    const n = new Map();
    for (const c of fn.calls) if (c.name && c.k !== 'ref') { const k = callee(c); n.set(k, (n.get(k) || []).concat(lineOf(c.pos))); }
    for (const [k, ls] of n) if (ls.length >= 3 && !/slog|fmt\.|errors\.|strings\.Builder|bytes\.Buffer|io\.StringWriter|Write|Lock|Unlock|Debug|Error|Warn|Info|Add|\.String$/.test(k)) console.log(`${at(fn)}\t${id(fn)}\t${k} called ${ls.length} times at lines ${ls.join(',')}`);
  }
} else if (query === 'unordered') {
  for (const d of own) for (const p of (d.facts && d.facts.unordered_iteration) || []) {
    const fn = all.find((f) => f.d === d && f.body && JSON.stringify(f.body).includes(`"pos":"${p}"`));
    console.log(`${rel(d)}${p}\t${fn ? id(fn) : '?'}\tfor_range over a map`);
  }
} else {
  console.error('unknown query: ' + query);
  process.exit(2);
}

// Classify the refusals of every cir/1 document in a directory by the construct that blocks it.
import fs from 'node:fs';
import path from 'node:path';

const dir = process.argv[2];
const kinds = new Map();
const add = (k, ex) => {
  const e = kinds.get(k) ?? {n: 0, ex: new Map()};
  e.n++;
  if (ex) e.ex.set(ex, (e.ex.get(ex) ?? 0) + 1);
  kinds.set(k, e);
};
const typeClass = (t) => {
  if (/^func\(/.test(t)) return ['func type', t.slice(0, 40)];
  if (/interface \{|^any$|^error$/.test(t)) return ['interface type', t];
  if (/^\[\d+\]/.test(t)) return ['fixed-size array', t];
  if (/float/.test(t)) return ['float', t];
  if (/uintptr|unsafe/.test(t)) return ['uintptr/unsafe', t];
  if (/^\*?\[?\]?\*?[\w/.-]+\.[A-Z]\w*$/.test(t.replace(/^[\[\]*]+/, '')) && t.includes('.')) {
    const m = t.match(/([\w/.-]+)\.([A-Z]\w*)$/);
    return ['type from another package', m ? m[1] + '.' + m[2] : t];
  }
  if (/struct \{/.test(t)) return ['anonymous struct', ''];
  return ['other type', t];
};
let total = 0;
for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.cir.json'))) {
  const m = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
  for (const r of m.refusals ?? []) {
    total++;
    const why = r.reason;
    let mm;
    if ((mm = why.match(/of type (.+) has no cir\/1 type$/)) || (mm = why.match(/^type outside cir\/1: (.+)$/)) || (mm = why.match(/underlying type (.+) has no/))) {
      const [k, ex] = typeClass(mm[1]);
      add(k, ex);
    } else if (/embedded field/.test(why)) add('embedded field', r.what);
    else if (/generic/.test(why)) add('generics', r.what);
    else if (/variadic/.test(why)) add('variadic function', r.what);
    else if (/without a Go body/.test(why)) add('no Go body', r.what);
    else if (/package-level variable/.test(why)) add('package-level variable', '');
    else if (/other than struct or basic newtype/.test(why)) add('type decl: interface, func, map, slice…', r.what);
    else if (/named results/.test(why)) add('named results', '');
    else if (/^statement \*ast\.(\w+)/.test(why)) add('statement ' + why.match(/^statement \*ast\.(\w+)/)[1], '');
    else add(why.slice(0, 60), '');
  }
}
console.log(`refusals: ${total}`);
for (const [k, e] of [...kinds].sort((a, b) => b[1].n - a[1].n)) {
  const ex = [...e.ex].sort((a, b) => b[1] - a[1]).slice(0, 4).map(([x, n]) => `${x} ${n}`).join('; ');
  console.log(`${String(e.n).padStart(4)}  ${k}${ex ? '  — ' + ex : ''}`);
}

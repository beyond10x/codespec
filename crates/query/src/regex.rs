//! The regular expressions of `~"…"`, searching anywhere in the string unless anchored.
//! Literals, `.`, `^`, `$`, classes `[a-z]` and `[^…]`, the escapes `\d \w \s \D \W \S` and any
//! escaped punctuation, groups `(…)` and `(?:…)`, alternation `|` and the quantifiers
//! `* + ? {n} {n,} {n,m}` (a trailing `?` is accepted; only whether a match exists is asked).
//!
//! An expression compiles to a program of at most [`MAX_PROGRAM`] instructions, groups nest at
//! most [`MAX_GROUPS`] deep and a count is at most [`MAX_COUNT`]; it runs as a Pike VM, every
//! thread advanced one character at a time, so matching takes time linear in the string and
//! never recurses.

/// How deep groups may nest.
pub const MAX_GROUPS: usize = 64;
/// The largest count `{n,m}` takes.
pub const MAX_COUNT: usize = 1000;
/// The most instructions an expression compiles to.
pub const MAX_PROGRAM: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    Char(char),
    Any,
    Class(Vec<(char, char)>, bool),
    Start,
    End,
    Group(Vec<Vec<Piece>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Piece {
    atom: Atom,
    min: usize,
    max: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Inst {
    Char(char),
    Any,
    Class(Vec<(char, char)>, bool),
    Start,
    End,
    Split(usize, usize),
    Jmp(usize),
    Match,
}

/// A compiled regular expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regex {
    prog: Vec<Inst>,
}

struct Parser<'a> {
    src: &'a str,
    chars: Vec<char>,
    i: usize,
    depth: usize,
}

const DIGIT: &[(char, char)] = &[('0', '9')];
const WORD: &[(char, char)] = &[('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')];
const SPACE: &[(char, char)] = &[(' ', ' '), ('\t', '\r')];

impl Parser<'_> {
    fn err(&self, what: &str) -> String {
        format!("regex {:?}: {what} at {}", self.src, self.i)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }

    fn alts(&mut self) -> Result<Vec<Vec<Piece>>, String> {
        let mut alts = vec![self.seq()?];
        while self.peek() == Some('|') {
            self.i += 1;
            alts.push(self.seq()?);
        }
        Ok(alts)
    }

    fn seq(&mut self) -> Result<Vec<Piece>, String> {
        let mut seq = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom()?;
            let (min, max) = self.quantifier()?;
            if (min, max) != (1, Some(1)) && matches!(atom, Atom::Start | Atom::End) {
                return Err(self.err("a quantified anchor"));
            }
            seq.push(Piece { atom, min, max });
        }
        Ok(seq)
    }

    fn number(&mut self) -> Result<Option<usize>, String> {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        if start == self.i {
            return Ok(None);
        }
        let n: Option<usize> = self.chars[start..self.i]
            .iter()
            .collect::<String>()
            .parse()
            .ok();
        match n {
            Some(n) if n <= MAX_COUNT => Ok(Some(n)),
            _ => Err(self.err(&format!("a count above {MAX_COUNT}, the limit of cq"))),
        }
    }

    fn quantifier(&mut self) -> Result<(usize, Option<usize>), String> {
        let q = match self.peek() {
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('?') => (0, Some(1)),
            Some('{') => {
                self.i += 1;
                let min = self.number()?.ok_or_else(|| self.err("a count"))?;
                let max = if self.peek() == Some(',') {
                    self.i += 1;
                    self.number()?
                } else {
                    Some(min)
                };
                if self.peek() != Some('}') {
                    return Err(self.err("an unclosed {"));
                }
                (min, max)
            }
            _ => return Ok((1, Some(1))),
        };
        self.i += 1;
        if self.peek() == Some('?') {
            self.i += 1;
        }
        if q.1.is_some_and(|m| m < q.0) {
            return Err(self.err("a count range whose maximum is below its minimum"));
        }
        Ok(q)
    }

    fn escape(&mut self) -> Result<Atom, String> {
        let c = self.peek().ok_or_else(|| self.err("a trailing \\"))?;
        self.i += 1;
        Ok(match c {
            'd' => Atom::Class(DIGIT.to_vec(), false),
            'D' => Atom::Class(DIGIT.to_vec(), true),
            'w' => Atom::Class(WORD.to_vec(), false),
            'W' => Atom::Class(WORD.to_vec(), true),
            's' => Atom::Class(SPACE.to_vec(), false),
            'S' => Atom::Class(SPACE.to_vec(), true),
            'n' => Atom::Char('\n'),
            't' => Atom::Char('\t'),
            c if c.is_ascii_alphanumeric() => return Err(self.err("an unknown escape")),
            c => Atom::Char(c),
        })
    }

    fn atom(&mut self) -> Result<Atom, String> {
        let c = self.peek().ok_or_else(|| self.err("an expression"))?;
        self.i += 1;
        Ok(match c {
            '.' => Atom::Any,
            '^' => Atom::Start,
            '$' => Atom::End,
            '\\' => self.escape()?,
            '(' => {
                if self.depth == MAX_GROUPS {
                    return Err(self.err(&format!(
                        "groups nested deeper than {MAX_GROUPS}, the limit of cq"
                    )));
                }
                if self.chars[self.i..].starts_with(&['?', ':']) {
                    self.i += 2;
                }
                self.depth += 1;
                let alts = self.alts()?;
                self.depth -= 1;
                if self.peek() != Some(')') {
                    return Err(self.err("an unclosed ("));
                }
                self.i += 1;
                Atom::Group(alts)
            }
            '[' => self.class()?,
            '*' | '+' | '?' | '{' => return Err(self.err("a quantifier with nothing before it")),
            c => Atom::Char(c),
        })
    }

    fn class(&mut self) -> Result<Atom, String> {
        let negated = self.peek() == Some('^');
        if negated {
            self.i += 1;
        }
        let mut ranges = Vec::new();
        let mut first = true;
        loop {
            let c = self.peek().ok_or_else(|| self.err("an unclosed ["))?;
            self.i += 1;
            if c == ']' && !first {
                break;
            }
            first = false;
            let lo = if c == '\\' {
                match self.escape()? {
                    Atom::Char(c) => c,
                    Atom::Class(r, false) => {
                        ranges.extend(r);
                        continue;
                    }
                    _ => return Err(self.err("a negated class inside a class")),
                }
            } else {
                c
            };
            if self.peek() == Some('-') && self.chars.get(self.i + 1).is_some_and(|&c| c != ']') {
                self.i += 1;
                let mut hi = self.peek().ok_or_else(|| self.err("an unclosed ["))?;
                self.i += 1;
                if hi == '\\' {
                    match self.escape()? {
                        Atom::Char(c) => hi = c,
                        _ => return Err(self.err("a class as a range end")),
                    }
                }
                if hi < lo {
                    return Err(self.err("a reversed range"));
                }
                ranges.push((lo, hi));
            } else {
                ranges.push((lo, lo));
            }
        }
        Ok(Atom::Class(ranges, negated))
    }
}

/// Compiles parsed alternatives into instructions; recursion follows the nesting of groups,
/// which the parser bounds.
struct Compiler {
    prog: Vec<Inst>,
}

impl Compiler {
    fn emit(&mut self, i: Inst) -> Result<usize, String> {
        if self.prog.len() == MAX_PROGRAM {
            return Err(format!(
                "regex: more than {MAX_PROGRAM} instructions, the limit of cq"
            ));
        }
        self.prog.push(i);
        Ok(self.prog.len() - 1)
    }

    fn alts(&mut self, alts: &[Vec<Piece>]) -> Result<(), String> {
        let mut jumps = Vec::new();
        for (k, seq) in alts.iter().enumerate() {
            if k + 1 < alts.len() {
                let split = self.emit(Inst::Split(0, 0))?;
                self.seq(seq)?;
                jumps.push(self.emit(Inst::Jmp(0))?);
                let next = self.prog.len();
                self.prog[split] = Inst::Split(split + 1, next);
            } else {
                self.seq(seq)?;
            }
        }
        let end = self.prog.len();
        for j in jumps {
            self.prog[j] = Inst::Jmp(end);
        }
        Ok(())
    }

    fn seq(&mut self, seq: &[Piece]) -> Result<(), String> {
        for p in seq {
            for _ in 0..p.min {
                self.atom(&p.atom)?;
            }
            match p.max {
                None => {
                    let split = self.emit(Inst::Split(0, 0))?;
                    self.atom(&p.atom)?;
                    self.emit(Inst::Jmp(split))?;
                    let out = self.prog.len();
                    self.prog[split] = Inst::Split(split + 1, out);
                }
                Some(max) => {
                    let mut splits = Vec::new();
                    for _ in p.min..max {
                        splits.push(self.emit(Inst::Split(0, 0))?);
                        self.atom(&p.atom)?;
                    }
                    let out = self.prog.len();
                    for s in splits {
                        self.prog[s] = Inst::Split(s + 1, out);
                    }
                }
            }
        }
        Ok(())
    }

    fn atom(&mut self, a: &Atom) -> Result<(), String> {
        match a {
            Atom::Char(c) => self.emit(Inst::Char(*c)).map(|_| ()),
            Atom::Any => self.emit(Inst::Any).map(|_| ()),
            Atom::Class(r, n) => self.emit(Inst::Class(r.clone(), *n)).map(|_| ()),
            Atom::Start => self.emit(Inst::Start).map(|_| ()),
            Atom::End => self.emit(Inst::End).map(|_| ()),
            Atom::Group(g) => self.alts(g),
        }
    }
}

impl Regex {
    pub fn new(src: &str) -> Result<Regex, String> {
        let mut p = Parser {
            src,
            chars: src.chars().collect(),
            i: 0,
            depth: 0,
        };
        let alts = p.alts()?;
        if p.i != p.chars.len() {
            return Err(p.err("an unopened )"));
        }
        let mut c = Compiler { prog: Vec::new() };
        c.alts(&alts)
            .map_err(|e| format!("regex {src:?}: {}", e.trim_start_matches("regex: ")))?;
        c.emit(Inst::Match)?;
        Ok(Regex { prog: c.prog })
    }

    /// Whether the expression matches somewhere in `s`.
    pub fn is_match(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        let n = chars.len();
        let mut mark = vec![usize::MAX; self.prog.len()];
        let mut stack = Vec::new();
        let mut cur: Vec<usize> = Vec::new();
        if self.add(&mut cur, &mut mark, &mut stack, 0, 0, n) {
            return true;
        }
        for (pos, &c) in chars.iter().enumerate() {
            let mut next = Vec::new();
            let step = pos + 1;
            for &pc in &cur {
                let hit = match &self.prog[pc] {
                    Inst::Char(x) => c == *x,
                    Inst::Any => true,
                    Inst::Class(ranges, negated) => {
                        ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi) != *negated
                    }
                    _ => false,
                };
                if hit && self.add(&mut next, &mut mark, &mut stack, pc + 1, step, n) {
                    return true;
                }
            }
            // A match may also start here.
            if self.add(&mut next, &mut mark, &mut stack, 0, step, n) {
                return true;
            }
            cur = next;
        }
        false
    }

    /// Adds the thread at `pc` to `list` at `pos`, following jumps, splits and anchors with an
    /// explicit stack; whether it reaches `Match`. `mark` holds the position each instruction was
    /// last added at, so each is added once per position.
    fn add(
        &self,
        list: &mut Vec<usize>,
        mark: &mut [usize],
        stack: &mut Vec<usize>,
        pc: usize,
        pos: usize,
        len: usize,
    ) -> bool {
        stack.clear();
        stack.push(pc);
        while let Some(pc) = stack.pop() {
            if mark[pc] == pos {
                continue;
            }
            mark[pc] = pos;
            match &self.prog[pc] {
                Inst::Match => return true,
                Inst::Jmp(x) => stack.push(*x),
                Inst::Split(a, b) => {
                    stack.push(*b);
                    stack.push(*a);
                }
                Inst::Start => {
                    if pos == 0 {
                        stack.push(pc + 1);
                    }
                }
                Inst::End => {
                    if pos == len {
                        stack.push(pc + 1);
                    }
                }
                _ => list.push(pc),
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Regex;

    fn m(re: &str, s: &str) -> bool {
        Regex::new(re).unwrap().is_match(s)
    }

    #[test]
    fn anchors_classes_groups_and_quantifiers_match_as_written() {
        assert!(m(
            r"^strconv\.(Atoi|ParseInt|ParseUint)$",
            "strconv.ParseInt"
        ));
        assert!(!m(
            r"^strconv\.(Atoi|ParseInt|ParseUint)$",
            "strconv.ParseIntX"
        ));
        assert!(!m(r"^strconv\.Atoi$", "strconvXAtoi"));
        assert!(m("^(<|<=|>|>=)$", "<="));
        assert!(!m("^(<|<=|>|>=)$", "=="));
        assert!(m("Read", "xxReadFrom"));
        assert!(m("^[&*]$", "*"));
        assert!(m(r"\.pb\.go:[0-9]+$", "x/a.pb.go:12"));
        assert!(!m(r"\.pb\.go:[0-9]+$", "x/apb.go:12"));
        assert!(m("^a{2,3}$", "aaa"));
        assert!(!m("^a{2,3}$", "aaaa"));
        assert!(m("^a{2,}$", "aaaaa"));
        assert!(!m("^a{2,}$", "a"));
        assert!(m("^(a*)*b$", "aaab"));
        assert!(!m("^(a*)*b$", "aaac"));
        assert!(m(r"^\w+\s\d$", "abc_1 7"));
        assert!(m("^[^a-c]$", "d"));
        assert!(!m("^[^a-c]$", "b"));
        assert!(m("^$", ""));
        assert!(m("", "x"));
        assert!(m("(^|/)zz", "a/zz"));
        assert!(m("(^|/)zz", "zz"));
        assert!(!m("(^|/)zz", "azz"));
    }

    #[test]
    fn a_broken_expression_is_refused() {
        for re in ["(a", "a)", "[a", "*a", r"\q", "a{3,1}", "a{1001}"] {
            assert!(Regex::new(re).is_err(), "{re}");
        }
        let deep = format!("{}a{}", "(".repeat(65), ")".repeat(65));
        assert!(Regex::new(&deep).unwrap_err().contains("limit of cq"));
    }

    #[test]
    fn a_long_string_is_matched_without_recursion() {
        let long = "a".repeat(200_000);
        assert!(m("^a*$", &long));
        assert!(!m("^a*b$", &long));
        assert!(m("^(a|aa)*$", &long));
    }
}

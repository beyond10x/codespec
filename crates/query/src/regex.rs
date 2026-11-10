//! The regular expressions of `~"…"`: a backtracking matcher over characters, searching anywhere
//! in the string unless anchored. Literals, `.`, `^`, `$`, classes `[a-z]` and `[^…]`, the escapes
//! `\d \w \s \D \W \S` and any escaped punctuation, groups `(…)` and `(?:…)`, alternation `|` and
//! the quantifiers `* + ? {n} {n,} {n,m}` (a trailing `?` is accepted; only whether a match
//! exists is asked).

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

/// A compiled regular expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regex {
    alts: Vec<Vec<Piece>>,
}

struct Parser<'a> {
    src: &'a str,
    chars: Vec<char>,
    i: usize,
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

    fn number(&mut self) -> Option<usize> {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        self.chars[start..self.i]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }

    fn quantifier(&mut self) -> Result<(usize, Option<usize>), String> {
        let q = match self.peek() {
            Some('*') => (0, None),
            Some('+') => (1, None),
            Some('?') => (0, Some(1)),
            Some('{') => {
                self.i += 1;
                let min = self.number().ok_or_else(|| self.err("a count"))?;
                let max = if self.peek() == Some(',') {
                    self.i += 1;
                    self.number()
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
                if self.chars[self.i..].starts_with(&['?', ':']) {
                    self.i += 2;
                }
                let alts = self.alts()?;
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

impl Regex {
    pub fn new(src: &str) -> Result<Regex, String> {
        let mut p = Parser {
            src,
            chars: src.chars().collect(),
            i: 0,
        };
        let alts = p.alts()?;
        if p.i != p.chars.len() {
            return Err(p.err("an unopened )"));
        }
        Ok(Regex { alts })
    }

    /// Whether the expression matches somewhere in `s`.
    pub fn is_match(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        (0..=chars.len()).any(|start| alts(&self.alts, &chars, start, &|_| true))
    }
}

fn alts(alts: &[Vec<Piece>], s: &[char], pos: usize, k: &dyn Fn(usize) -> bool) -> bool {
    alts.iter().any(|seq| sequence(seq, s, pos, k))
}

fn sequence(seq: &[Piece], s: &[char], pos: usize, k: &dyn Fn(usize) -> bool) -> bool {
    match seq.split_first() {
        None => k(pos),
        Some((p, rest)) => piece(p, 0, s, pos, &|next| sequence(rest, s, next, k)),
    }
}

fn piece(p: &Piece, count: usize, s: &[char], pos: usize, k: &dyn Fn(usize) -> bool) -> bool {
    if p.max.is_none_or(|m| count < m)
        && atom(&p.atom, s, pos, &|next| {
            (next != pos || count < p.min) && piece(p, count + 1, s, next, k)
        })
    {
        return true;
    }
    count >= p.min && k(pos)
}

fn atom(a: &Atom, s: &[char], pos: usize, k: &dyn Fn(usize) -> bool) -> bool {
    match a {
        Atom::Start => pos == 0 && k(pos),
        Atom::End => pos == s.len() && k(pos),
        Atom::Group(g) => alts(g, s, pos, k),
        _ => {
            let Some(&c) = s.get(pos) else { return false };
            let hit = match a {
                Atom::Char(x) => c == *x,
                Atom::Any => true,
                Atom::Class(ranges, negated) => {
                    ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi) != *negated
                }
                _ => unreachable!("handled above"),
            };
            hit && k(pos + 1)
        }
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
        assert!(m("^(a*)*b$", "aaab"));
        assert!(!m("^(a*)*b$", "aaac"));
        assert!(m(r"^\w+\s\d$", "abc_1 7"));
        assert!(m("^[^a-c]$", "d"));
        assert!(!m("^[^a-c]$", "b"));
        assert!(m("^$", ""));
    }

    #[test]
    fn a_broken_expression_is_refused() {
        for re in ["(a", "a)", "[a", "*a", r"\q", "a{3,1}"] {
            assert!(Regex::new(re).is_err(), "{re}");
        }
    }
}

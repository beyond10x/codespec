//! What the refusals of [`crate::checks`] read from a file's bytes, for Rust and Go: tokens
//! (comments skipped, string, character and rune literals as one token each), the region a
//! reader's refusal at a line covers, the trait of the `impl` block around a byte, a Rust method's
//! receiver mode, a struct's attributes and whether a Go struct has a field tag. Pure: the bytes
//! are given.

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An identifier or keyword.
    Ident,
    /// A string, raw string, character, byte or rune literal.
    Str,
    /// A number.
    Num,
    /// A Rust lifetime or label (`'a`).
    Lifetime,
    /// One byte of punctuation.
    Punct(u8),
}

/// One token: its kind, its bytes `start..end` and the 1-based line of `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tok {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub line: usize,
}

impl Tok {
    /// The token's text in `b`.
    pub fn text<'b>(&self, b: &'b [u8]) -> &'b str {
        std::str::from_utf8(&b[self.start..self.end]).unwrap_or("")
    }

    fn is(&self, p: u8) -> bool {
        self.kind == Kind::Punct(p)
    }

    fn ident(&self, b: &[u8], word: &str) -> bool {
        self.kind == Kind::Ident && self.text(b) == word
    }
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c >= 0x80
}

fn ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80
}

/// Whether `c` is part of an identifier or a number token: what two tokens may not be joined
/// across.
pub fn word_byte(c: u8) -> bool {
    ident_byte(c)
}

/// The tokens of `b`, Rust or Go source, comments skipped. Never fails: an unterminated literal
/// or comment runs to the end.
pub fn tokens(b: &[u8]) -> Vec<Tok> {
    let mut out = Vec::new();
    let (mut i, mut line) = (0usize, 1usize);
    let n = b.len();
    // The end of a quoted literal opened at `i` (the quote at `i`), escapes skipped.
    let quoted = |mut j: usize, q: u8| {
        j += 1;
        while j < n && b[j] != q {
            if b[j] == b'\\' {
                j += 1;
            }
            j += 1;
        }
        (j + 1).min(n)
    };
    while i < n {
        let c = b[i];
        let start = i;
        let at_line = line;
        if c.is_ascii_whitespace() {
            if c == b'\n' {
                line += 1;
            }
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < n && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            let mut depth = 0usize;
            while i < n {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                    depth += 1;
                    i += 2;
                } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if b[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            continue;
        }
        let kind;
        if ident_start(c) {
            let mut j = i;
            while j < n && ident_byte(b[j]) {
                j += 1;
            }
            let word = &b[i..j];
            // Rust raw strings and byte or C strings: r"…", r#"…"#, br"…", b"…", c"…".
            let raw = matches!(word, b"r" | b"br" | b"cr");
            if raw && j < n && (b[j] == b'"' || b[j] == b'#') {
                let mut k = j;
                while k < n && b[k] == b'#' {
                    k += 1;
                }
                if k < n && b[k] == b'"' {
                    let hashes = k - j;
                    let mut e = k + 1;
                    loop {
                        if e >= n {
                            break;
                        }
                        if b[e] == b'"'
                            && b[e + 1..]
                                .iter()
                                .take(hashes)
                                .filter(|h| **h == b'#')
                                .count()
                                == hashes
                        {
                            e += 1 + hashes;
                            break;
                        }
                        e += 1;
                    }
                    i = e.min(n);
                    kind = Kind::Str;
                    line += b[start..i].iter().filter(|x| **x == b'\n').count();
                    out.push(Tok {
                        kind,
                        start,
                        end: i,
                        line: at_line,
                    });
                    continue;
                }
            }
            if matches!(word, b"b" | b"c") && j < n && (b[j] == b'"' || b[j] == b'\'') {
                i = quoted(j, b[j]);
                kind = Kind::Str;
            } else {
                i = j;
                kind = Kind::Ident;
            }
        } else if c.is_ascii_digit() {
            let mut j = i;
            while j < n
                && (ident_byte(b[j])
                    || b[j] == b'.' && b.get(j + 1).is_some_and(u8::is_ascii_digit))
            {
                j += 1;
            }
            i = j;
            kind = Kind::Num;
        } else if c == b'"' {
            i = quoted(i, b'"');
            kind = Kind::Str;
        } else if c == b'`' {
            let mut j = i + 1;
            while j < n && b[j] != b'`' {
                j += 1;
            }
            i = (j + 1).min(n);
            kind = Kind::Str;
        } else if c == b'\'' {
            // A character or rune literal, or a Rust lifetime or label.
            if b.get(i + 1) == Some(&b'\\') {
                i = quoted(i, b'\'');
                kind = Kind::Str;
            } else {
                let width = b.get(i + 1).map_or(1, |x| match x {
                    0xf0..=0xff => 4,
                    0xe0..=0xef => 3,
                    0xc0..=0xdf => 2,
                    _ => 1,
                });
                if b.get(i + 1 + width) == Some(&b'\'') {
                    i += 2 + width;
                    kind = Kind::Str;
                } else {
                    let mut j = i + 1;
                    while j < n && ident_byte(b[j]) {
                        j += 1;
                    }
                    i = j.max(i + 1);
                    kind = Kind::Lifetime;
                }
            }
        } else {
            i += 1;
            kind = Kind::Punct(c);
        }
        line += b[start..i].iter().filter(|x| **x == b'\n').count();
        out.push(Tok {
            kind,
            start,
            end: i,
            line: at_line,
        });
    }
    out
}

/// The bytes of the region a reader's refusal at `line` covers: from the first token on that line
/// or after it, to the end of the construct starting there. It ends after a `;` outside brackets,
/// before a closing bracket it did not open (the end of the block around it), after a `}` that
/// closes its last open bracket when the next token is on a later line, and for Go before a token
/// on a later line where a line break ends a statement. `None` when no token is at or after
/// `line`.
pub fn region(b: &[u8], toks: &[Tok], line: usize, go: bool) -> Option<(usize, usize)> {
    let first = toks.iter().position(|t| t.line >= line)?;
    let mut depth = 0usize;
    let mut end = toks[first].end;
    for (i, t) in toks.iter().enumerate().skip(first) {
        if go
            && i > first
            && depth == 0
            && t.line > toks[i - 1].line
            && ends_go_line(b, &toks[i - 1])
        {
            break;
        }
        match t.kind {
            Kind::Punct(b'(' | b'[' | b'{') => depth += 1,
            Kind::Punct(c @ (b')' | b']' | b'}')) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if depth == 0 && c == b'}' && toks.get(i + 1).is_none_or(|n| n.line > t.line) {
                    end = t.end;
                    break;
                }
            }
            Kind::Punct(b';') if depth == 0 => {
                end = t.end;
                break;
            }
            _ => {}
        }
        end = t.end;
    }
    Some((toks[first].start, end))
}

/// Whether a Go line break after `t` ends a statement (The Go Programming Language Specification,
/// § Semicolons).
fn ends_go_line(b: &[u8], t: &Tok) -> bool {
    match t.kind {
        Kind::Ident => !matches!(
            t.text(b),
            "func"
                | "if"
                | "for"
                | "switch"
                | "select"
                | "else"
                | "go"
                | "defer"
                | "var"
                | "const"
                | "type"
                | "import"
                | "package"
                | "case"
                | "chan"
                | "map"
                | "struct"
                | "interface"
                | "range"
        ),
        Kind::Str | Kind::Num => true,
        Kind::Punct(c) => matches!(c, b')' | b']' | b'}'),
        Kind::Lifetime => false,
    }
}

/// The identifiers among the tokens inside `start..end`.
pub fn idents<'b>(b: &'b [u8], toks: &[Tok], (start, end): (usize, usize)) -> Vec<&'b str> {
    toks.iter()
        .filter(|t| t.kind == Kind::Ident && t.start >= start && t.end <= end)
        .map(|t| t.text(b))
        .collect()
}

/// The trait of the `impl` block directly around byte `at` (a Rust method's name): the last
/// identifier of the path between `impl` (and its generics) and `for`, its generic arguments
/// left out. `None` when the block around `at` is no `impl … for …`.
pub fn impl_trait(b: &[u8], toks: &[Tok], at: usize) -> Option<String> {
    // Per open brace: the tokens before it since the last `;`, `{` or `}` outside parentheses.
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut since = 0usize;
    let mut parens = 0usize;
    for (i, t) in toks.iter().enumerate() {
        if t.start >= at {
            break;
        }
        match t.kind {
            Kind::Punct(b'(' | b'[') => parens += 1,
            Kind::Punct(b')' | b']') => parens = parens.saturating_sub(1),
            Kind::Punct(b'{') => {
                stack.push((since, i));
                since = i + 1;
                parens = 0;
            }
            Kind::Punct(b'}') => {
                stack.pop();
                since = i + 1;
                parens = 0;
            }
            Kind::Punct(b';') if parens == 0 => since = i + 1,
            _ => {}
        }
    }
    let &(from, to) = stack.last()?;
    let head = skip_attributes(&toks[from..to]);
    let mut k = 0;
    while k < head.len() && (head[k].ident(b, "unsafe") || head[k].ident(b, "default")) {
        k += 1;
    }
    if !head.get(k)?.ident(b, "impl") {
        return None;
    }
    k += 1;
    if head.get(k).is_some_and(|t| t.is(b'<')) {
        k = after_angles(head, k);
    }
    let mut angles = 0usize;
    let mut name = None;
    for t in &head[k..] {
        match t.kind {
            Kind::Punct(b'<') => angles += 1,
            Kind::Punct(b'>') => angles = angles.saturating_sub(1),
            Kind::Ident if angles == 0 && t.text(b) == "for" => return name,
            Kind::Ident if angles == 0 => name = Some(t.text(b).to_string()),
            _ => {}
        }
    }
    None
}

/// The tokens after the `<…>` opening at `k`: the index past its closing `>`.
fn after_angles(toks: &[Tok], mut k: usize) -> usize {
    let mut depth = 0usize;
    while k < toks.len() {
        if toks[k].is(b'<') {
            depth += 1;
        } else if toks[k].is(b'>') {
            depth -= 1;
            if depth == 0 {
                return k + 1;
            }
        }
        k += 1;
    }
    k
}

/// `toks` without the outer attributes (`#[…]`) it starts with.
fn skip_attributes(mut toks: &[Tok]) -> &[Tok] {
    while toks.len() > 1 && toks[0].is(b'#') && toks[1].is(b'[') {
        let mut depth = 0usize;
        let mut k = 1;
        while k < toks.len() {
            if toks[k].is(b'[') {
                depth += 1;
            } else if toks[k].is(b']') {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            k += 1;
        }
        toks = &toks[(k + 1).min(toks.len())..];
    }
    toks
}

/// The receiver mode of the Rust method whose name token ends at `name_end`, as a type:
/// `Self` (`self`, `mut self`, `self: Self`), `&Self` (`&self`, `&'a self`, `self: &Self`),
/// `&mut Self`, or the written type (`Box<Self>`), lifetimes left out. `None` for a function
/// without a `self` parameter.
pub fn receiver_mode(b: &[u8], toks: &[Tok], name_end: usize) -> Option<String> {
    let mut k = toks.iter().position(|t| t.start >= name_end)?;
    if toks[k].is(b'<') {
        k = after_angles(toks, k);
    }
    if !toks.get(k)?.is(b'(') {
        return None;
    }
    let mut param = Vec::new();
    let mut depth = 0usize;
    for t in &toks[k + 1..] {
        match t.kind {
            Kind::Punct(b'(' | b'[' | b'<') => depth += 1,
            Kind::Punct(b')' | b']' | b'>') if depth > 0 => depth -= 1,
            Kind::Punct(b')' | b',') if depth == 0 => break,
            _ => {}
        }
        if t.kind != Kind::Lifetime {
            param.push(*t);
        }
    }
    let param = skip_attributes(&param);
    let at = param.iter().position(|t| t.ident(b, "self"))?;
    let words = |ts: &[Tok]| -> String {
        let mut s = String::new();
        for t in ts {
            if t.kind == Kind::Ident && s.ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                s.push(' ');
            }
            s.push_str(t.text(b));
        }
        s
    };
    if param.get(at + 1).is_some_and(|t| t.is(b':')) {
        return Some(words(&param[at + 2..]));
    }
    let before = &param[..at];
    Some(if before.first().is_some_and(|t| t.is(b'&')) {
        if before.iter().any(|t| t.ident(b, "mut")) {
            "&mut Self".into()
        } else {
            "&Self".into()
        }
    } else {
        "Self".into()
    })
}

/// The `struct <name>` declaration at or after `line`: the index of its `struct` token (Rust) or
/// of its name (Go, `<name> struct`).
fn struct_at(b: &[u8], toks: &[Tok], line: usize, name: &str, go: bool) -> Option<usize> {
    let from = toks.iter().position(|t| t.line >= line)?;
    (from..toks.len().saturating_sub(1)).find(|&i| {
        if go {
            toks[i].ident(b, name) && toks[i + 1].ident(b, "struct")
        } else {
            toks[i].ident(b, "struct") && toks[i + 1].ident(b, name)
        }
    })
}

/// The attributes of the Rust struct `name` declared at or after `line`: each `#[…]` before its
/// `struct` keyword, as the path it starts with and, for `derive`, each derived trait's last path
/// segment. `None` when the declaration is not found.
pub fn struct_attributes(
    b: &[u8],
    toks: &[Tok],
    line: usize,
    name: &str,
) -> Option<Vec<(String, Vec<String>)>> {
    let at = struct_at(b, toks, line, name, false)?;
    // Back over visibility (`pub`, `pub(crate)`), then each attribute.
    let mut k = at;
    if k > 0 && toks[k - 1].is(b')') {
        while k > 0 && !toks[k - 1].is(b'(') {
            k -= 1;
        }
        k = k.saturating_sub(1);
    }
    if k > 0 && toks[k - 1].ident(b, "pub") {
        k -= 1;
    }
    let mut out = Vec::new();
    while k >= 3 && toks[k - 1].is(b']') {
        let mut depth = 0usize;
        let mut j = k - 1;
        loop {
            if toks[j].is(b']') {
                depth += 1;
            } else if toks[j].is(b'[') {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            if j == 0 {
                return Some(out);
            }
            j -= 1;
        }
        if j == 0 || !toks[j - 1].is(b'#') {
            break;
        }
        let inner = &toks[j + 1..k - 1];
        let path = inner
            .iter()
            .take_while(|t| t.kind == Kind::Ident || t.is(b':'))
            .map(|t| t.text(b))
            .collect::<String>();
        let derived = if path == "derive" {
            // Each comma-separated path's last identifier.
            let mut names = Vec::new();
            let mut last = None;
            for t in inner.iter().skip(1) {
                match t.kind {
                    Kind::Ident => last = Some(t.text(b).to_string()),
                    Kind::Punct(b',' | b')') => names.extend(last.take()),
                    _ => {}
                }
            }
            names
        } else {
            Vec::new()
        };
        out.push((path, derived));
        k = j - 1;
    }
    out.reverse();
    Some(out)
}

/// Whether the Go struct `name` declared at or after `line` has a field tag: a string literal
/// inside its braces. `None` when the declaration is not found.
pub fn go_struct_has_tag(b: &[u8], toks: &[Tok], line: usize, name: &str) -> Option<bool> {
    let at = struct_at(b, toks, line, name, true)?;
    let open = at + 2;
    if !toks.get(open)?.is(b'{') {
        return Some(false);
    }
    let mut depth = 0usize;
    for t in &toks[open..] {
        match t.kind {
            Kind::Punct(b'{') => depth += 1,
            Kind::Punct(b'}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(false);
                }
            }
            Kind::Str => return Some(true),
            _ => {}
        }
    }
    Some(false)
}

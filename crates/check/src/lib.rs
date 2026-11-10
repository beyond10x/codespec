//! Built-in defect checks over a set of `cir/1` documents, read as one program. `codespec check`
//! runs one of them and prints its findings.
//!
//! A check reads only `cir/1`: read bodies, the call lists of `foreign` statements and opaque
//! functions, method sets and the module's types. It never reads source text. A function is
//! either read (its body) or opaque (its call list); every check reads both, as far as each shows
//! what it looks for, and says below which it needs.
//!
//! # Calls and names
//!
//! A function's calls are those of its body's expressions and of its `foreign` statements' call
//! lists, function literals included, ordered by line; an opaque function's are its call list. A
//! call names `path.name` (a function), `path.Recv.name` (a method of a declared type or an
//! interface; the caller's own module gives its own path). A list entry names a call when it
//! equals its name, `Recv.name`, `path.name` or `path.Recv.name`; an entry ending in `*` names
//! every call one of those four starts with the rest of it (`Delete*`, `sync.WaitGroup.*`). The
//! lists are data in [`CHECKS`], printed by `codespec check --list` and replaced one by one by a
//! names file, a JSON object of list name to array of entries ([`Names::from_json`]).
//!
//! The static call graph links a call to the function of the set it names (a function, a method
//! of a declared type, a function used as a value); a call through an interface or a function
//! value links to nothing. A function's words are its name split at `_` and at case changes
//! (`NewSessionID`: `new`, `session`, `id`), lower case, a final `s` dropped.
//!
//! # The checks
//!
//! - `unbounded-growth`: a function that calls a `receive` method (by default, a receive from the
//!   network; a function of the set `receive-wrappers` names that calls a receive is one too, to
//!   a fixed point), then, at a later line, an
//!   `insert` method of a type (a method or an interface call; not one `not-containers` names),
//!   with no `remove` method of that same type called after the receive in the function or in any
//!   function the calls after the receive reach. Name based: whether the key comes from the input
//!   is not followed.
//! - `insecure-random`: a call into a `random` package from a function reached, by the static
//!   call graph, from a function one of whose words is in `token` (the function itself included);
//!   with `--strict`, every call into a `random` package. Calls inside the `random` packages
//!   themselves are not reported. When such a call lies in a module `--report` leaves out (a
//!   dependency), a reported function's call into that module that reaches a `token` function
//!   that reaches it is reported instead, at the call.
//! - `int-from-text`: in a read body, `v := <parse>(…)` (a `let` or an `assign`, into a local or
//!   a field), with no `<`, `<=`, `>` or `>=` on `v` anywhere in the function, and either a
//!   conversion of `v` to an integer type narrower than what the parse can give (its result type,
//!   or its bit size argument when that is a constant; a declared newtype or enum resolved to its
//!   integer type), or a word of `v`'s name, or of a local or field `v` is copied into, in
//!   `ranged` (`port`, `status`): no comparison then covers the copies too. Go's conversion to an
//!   unnamed integer type is outside `cir/1` and so not seen.
//! - `unsigned-below-zero`: in a read body, an unsigned integer compared with `0` by `<` (never
//!   true), `<=` (true only at 0) or `>=` (always true), either way round: the comparison is
//!   written as if the value could go below zero, where it wraps to its largest value instead.
//! - `case-fold-asymmetry`: two sibling methods of one type, where one calls a fold and the other
//!   calls none and compares two strings, neither a constant, with `==` or `!=` (read body), or,
//!   when its body is opaque or holds `foreign` statements, calls a method its folding sibling
//!   calls on the line of a fold. A fold is a call `fold` names, or of a function of the set one
//!   of whose words is in `fold-words` (`HeaderToLower`), or of a plain function from one string
//!   to a string that calls a fold, to a fixed point. Siblings share a word of their names other
//!   than the first (`GetHeader`, `RemoveHeader`); methods of one word are all siblings.
//! - `clone-aliasing`: sibling clone functions (a word in `clone`; one name once every type name
//!   of the module in it is replaced, or one method name on two types), where one calls a `copy`
//!   function and another calls none and passes a slice through: a struct literal field or a field
//!   assignment whose value is a slice read from a field or a method (read body), or a method its
//!   copying sibling calls on the line of its copy (call list).
//! - `writer-bypass`: in a read body with a parameter of a `writers` type, a `Write…` call on that
//!   parameter whose argument is the result of a method `M` of a declared type `T`, where `T` has
//!   another method that takes a writer and either `M` calls it (`M` builds its string with it) or
//!   it is named `M` + `Write` (`ToString`, `ToStringWrite`): the caller builds an intermediate
//!   string the writer form would not.
//! - `dropped-error`: in a read body, a call whose last result is `error` (Go `error`) used as a
//!   statement, its result unused, or bound by a `let`/`assign` whose last name is `_`; not one
//!   `ignore` names, and not a call on a value of a `never-fail` type (`strings.Builder`), nor on
//!   a parameter every call of the set fills with such a value or such a parameter of its caller
//!   (a greatest fixed point; a call through an interface counts for every method of its name; a
//!   function used as a value, or called from an opaque function or a `foreign` statement, has no
//!   such parameter). The `fallible` fact of `codespec pass` (`crates/passes`) is a subset of
//!   these callees: the evidence says when the callee is listed there.
//! - `dropped-reader`: in a read body, a result of a `buffered` type (`bufio.Reader`) a call
//!   gives, bound to `_` or left unused: the bytes it read ahead of what its caller consumed are
//!   lost with it.
//! - `error-eq`: in a read body, `==` or `!=` between a sentinel error (a module error or an
//!   error-typed global) and an error from a call `unwrapped` names (`context.Cause`), or from a
//!   function of the set that wraps errors (an `errorf` with `%w`, `errors.Join`), directly or
//!   through the local it was bound to: a wrapped sentinel does not match.
//! - `lock-without-unlock`: in a read body, a `lock` call with a path to a `return`, a `raise`, a
//!   call of a function of the set that lists `raises` (the path may leave there), or the end of
//!   the function, that passes no matching `unlock` (the entry at the same index). Locks are
//!   matched by kind (`sync.Mutex.Lock` with `sync.Mutex.Unlock`), not by receiver. An unlock in a
//!   `foreign` statement (`defer`, a deferred function literal) releases there and on; so does a
//!   call of a function of the set that unlocks for its caller (its calls, in line order, unlock
//!   a kind with no lock of it taken before, itself or through such a callee). An unlock with no
//!   lock of its kind held releases the caller's lock, and the next lock of the kind takes it
//!   back for the caller. `if`, `switch` and loops fork the paths and join them; a loop body is
//!   followed once; a lock taken in one branch of an `if` is not held in the other branch of a
//!   later `if` on the same condition. An opaque function has no paths and is not checked. A
//!   function one of whose words is in `holders` (`lockProfiles`, `acquireForkLock`) holds the
//!   lock for its caller and is not checked.
//! - `timer-not-stopped`: in read bodies, a method of a type that stops (`stop`) a timer held in
//!   a field of its receiver, itself or through a method of the type it calls, and arms none, while
//!   another method of the type arms (`arm`) a timer into a field it does not stop: that timer
//!   outlives the teardown.
//! - `conn-leak`: in a read body (and each function literal in it, on its own), a local bound to
//!   a call whose first result is a `resources` type, on a path to a `return` of a non-nil error
//!   that neither `close`s it, returns it, nor hands it on (passes it to a call, stores it, or
//!   captures it); a call that takes it and gives a resource wraps it, and the wrapper is followed
//!   instead. In the branch of `if err != nil` on the error the opening call gave, it is nil. A
//!   `foreign` statement may do anything with it, and ends the following.
//! - `accept-blocking`: a function whose calls (read or opaque) hold an `accept`, then a
//!   `blocking` call at a later line, not inside a function literal, and no `deadline` call: one
//!   peer that connects and sends nothing stops every later accept.
//! - `after-in-loop`: in a read body, an `after` call (`time.After`) in a loop, or in a top-level
//!   `foreign` statement (a loop `cir/1` keeps foreign is one), that also calls a `periodic`
//!   rearm (`time.Timer.Reset`): the one-shot wait starts over on every turn and never ends
//!   while the timer fires.
//! - `frame-not-consumed`: in a read loop body, an `open` call (a frame header read whose payload
//!   the caller must read or skip) with a path to a `continue`, or to the end of the loop body,
//!   that passes no `consume` call: the next header is read from the payload.
//! - `quadratic-insert`: in a read body, a call inside a loop of an `insert-scan` method that
//!   ranges over its receiver or a field of it, itself or through a method of its own type it
//!   calls: n inserts cost n² steps.
//! - `unreached`: every function no entry reaches, as `codespec reach` gives it (`rta`): the
//!   entries `--entry` and `--exported` name, or every `main.main`, and every `init`. A module no
//!   function of which but its `init` is reached is not part of the program and is not reported
//!   (every `init` of the set is an entry, whether anything imports its module or not); a set with
//!   no entry is refused. A function in a file `generated` names (`*.pb.go`) is not reported:
//!   `cir/1` keeps no mark of a generated file, so its name decides. A `generated` entry starting
//!   with `*` names every file name ending in the rest.
//!
//! # Findings
//!
//! A finding is a position (`<module path>/<file>:<line>`), the function (`<module path>.<key>`),
//! the check, and the evidence: what the check matched, each with its position. The text output is
//! one line per finding, `<pos> <function> <check>: <evidence>`, the evidence items joined by `; `
//! each as `<what> at <pos>`. `--format json` prints the same findings as
//! `codespec-check-report/1` ([`Report`]). Findings are sorted by position, then function; the same
//! set gives the same bytes. `--report` keeps the findings of the modules whose path is or lies
//! under one given; the checks still read the whole set.

mod checks;
mod resources;
mod set;
mod values;

use std::collections::BTreeMap;

use codespec_model::Module;
use serde::{Deserialize, Serialize};

/// The format of the report [`run`] gives.
pub const FORMAT: &str = "codespec-check-report/1";

/// A list of names a check matches calls or function names against.
pub struct List {
    pub name: &'static str,
    /// What the list holds, for `--list`.
    pub what: &'static str,
    pub default: &'static [&'static str],
}

/// One built-in check.
pub struct Check {
    pub name: &'static str,
    /// One line on what it finds.
    pub finds: &'static str,
    /// More than half its findings on the test repository were false: see
    /// `docs/checks-capslock.md`.
    pub experimental: bool,
    pub lists: &'static [List],
    run: fn(&checks::Ctx) -> Result<Vec<Finding>, String>,
}

/// Every check, in the order `--list` prints them.
pub const CHECKS: &[Check] = &[
    Check {
        name: "unbounded-growth",
        finds: "a function that receives from the network, then inserts into a container type with no remove of that type after the receive",
        experimental: false,
        lists: &[
            List {
                name: "receive",
                what: "calls that receive from the network",
                default: &[
                    "net.Conn.Read",
                    "net.PacketConn.ReadFrom",
                    "net.Listener.Accept",
                    "net.TCPConn.Read",
                    "net.UDPConn.Read",
                    "net.UDPConn.ReadFrom",
                    "net.UDPConn.ReadFromUDP",
                    "net.UDPConn.ReadFromUDPAddrPort",
                    "net.UDPConn.ReadMsgUDP",
                    "net.UDPConn.ReadMsgUDPAddrPort",
                    "net.UnixConn.Read",
                    "net.UnixConn.ReadFrom",
                    "net.UnixConn.ReadFromUnix",
                    "net.UnixConn.ReadMsgUnix",
                    "net.IPConn.Read",
                    "net.IPConn.ReadFrom",
                    "net.IPConn.ReadFromIP",
                    "net.IPConn.ReadMsgIP",
                    "net.TCPListener.Accept",
                    "net.TCPListener.AcceptTCP",
                    "net.UnixListener.Accept",
                    "net.UnixListener.AcceptUnix",
                    "crypto/tls.Conn.Read",
                ],
            },
            List {
                name: "receive-wrappers",
                what: "names of functions of the set that receive when they call a receive",
                default: &["Read*", "Accept*", "Recv*"],
            },
            List {
                name: "insert",
                what: "methods that add an entry to a container",
                default: &[
                    "Add",
                    "Put",
                    "Store",
                    "Set",
                    "Insert",
                    "LoadOrStore",
                    "Push",
                    "add",
                    "put",
                    "store",
                    "set",
                    "insert",
                    "push",
                ],
            },
            List {
                name: "remove",
                what: "methods that take an entry out of a container",
                default: &[
                    "Delete*",
                    "Remove*",
                    "Evict*",
                    "Clear*",
                    "Drop*",
                    "Pop*",
                    "LoadAndDelete",
                    "CompareAndDelete",
                    "Done",
                    "delete*",
                    "remove*",
                    "evict*",
                    "clear*",
                    "drop*",
                    "pop*",
                ],
            },
            List {
                name: "not-containers",
                what: "types whose insert-named methods add no entry",
                default: &[
                    "sync.WaitGroup.*",
                    "sync.Pool.*",
                    "sync/atomic.*",
                    "net/http.Header.*",
                    "net/url.Values.*",
                    "net/textproto.MIMEHeader.*",
                ],
            },
        ],
        run: checks::unbounded_growth,
    },
    Check {
        name: "insecure-random",
        finds: "a math/rand call reached from a function named for a token, key, nonce or id (every math/rand call with --strict)",
        experimental: false,
        lists: &[
            List {
                name: "random",
                what: "packages whose output can be predicted",
                default: &["math/rand", "math/rand/v2"],
            },
            List {
                name: "token",
                what: "words of a function name that make a secret or an identifier",
                default: &[
                    "token",
                    "key",
                    "nonce",
                    "id",
                    "uuid",
                    "secret",
                    "salt",
                    "session",
                    "password",
                    "cookie",
                    "otp",
                    "csrf",
                    "mask",
                    "challenge",
                ],
            },
        ],
        run: checks::insecure_random,
    },
    Check {
        name: "int-from-text",
        finds: "an integer parsed from text, then converted to a narrower integer type or kept in a name for a ranged value (a port), with no range comparison in the function",
        experimental: false,
        lists: &[
            List {
                name: "parse",
                what: "functions that parse an integer from text",
                default: &["strconv.Atoi", "strconv.ParseInt", "strconv.ParseUint"],
            },
            List {
                name: "ranged",
                what: "words of a variable or field name whose integer has a range narrower than its type",
                default: &["port", "status"],
            },
        ],
        run: checks::int_from_text,
    },
    Check {
        name: "unsigned-below-zero",
        finds: "an unsigned integer compared with 0 by <, <=, >= or >: always false, == 0, or always true, where a value below zero was meant",
        experimental: false,
        lists: &[],
        run: values::unsigned_below_zero,
    },
    Check {
        name: "case-fold-asymmetry",
        finds: "sibling methods of one type where one compares names after case folding and another with ==",
        experimental: false,
        lists: &[
            List {
                name: "fold",
                what: "functions that fold case",
                default: &[
                    "strings.ToLower",
                    "strings.ToUpper",
                    "strings.EqualFold",
                    "bytes.ToLower",
                    "bytes.ToUpper",
                    "bytes.EqualFold",
                    "unicode.ToLower",
                    "unicode.ToUpper",
                    "net/textproto.CanonicalMIMEHeaderKey",
                    "net/http.CanonicalHeaderKey",
                ],
            },
            List {
                name: "fold-words",
                what: "words of the name of a function of the set that make it fold case",
                default: &["lower", "upper", "fold", "canonical"],
            },
        ],
        run: checks::case_fold_asymmetry,
    },
    Check {
        name: "clone-aliasing",
        finds: "sibling clone functions where one copies a slice and another passes one through",
        experimental: false,
        lists: &[
            List {
                name: "clone",
                what: "words of a function name that make it a clone function",
                default: &["clone", "copy", "dup", "duplicate"],
            },
            List {
                name: "copy",
                what: "calls that copy a slice or the value they are called on",
                default: &[
                    "slices.Clone",
                    "bytes.Clone",
                    "maps.Clone",
                    "Clone",
                    "clone",
                ],
            },
        ],
        run: checks::clone_aliasing,
    },
    Check {
        name: "writer-bypass",
        finds: "a function with a writer that writes the string a method builds, where that method builds it with a method of its type that takes a writer",
        experimental: false,
        lists: &[List {
            name: "writers",
            what: "types a function writes its output into",
            default: &[
                "io.Writer",
                "io.StringWriter",
                "strings.Builder",
                "bytes.Buffer",
                "bufio.Writer",
            ],
        }],
        run: values::writer_bypass,
    },
    Check {
        name: "dropped-error",
        finds: "a call whose error result is not used or is assigned to _",
        experimental: true,
        lists: &[
            List {
                name: "ignore",
                what: "calls whose error may be dropped",
                default: &[
                    "fmt.Print",
                    "fmt.Printf",
                    "fmt.Println",
                    "fmt.Fprint",
                    "fmt.Fprintf",
                    "fmt.Fprintln",
                ],
            },
            List {
                name: "never-fail",
                what: "types whose methods never return a non-nil error: a call on one, or on a parameter every caller fills with one, may drop it",
                default: &[
                    "strings.Builder",
                    "bytes.Buffer",
                    "hash.Hash",
                    "hash.Hash32",
                    "hash.Hash64",
                ],
            },
        ],
        run: checks::dropped_error,
    },
    Check {
        name: "dropped-reader",
        finds: "a buffered reader a call returns, assigned to _ or unused: the bytes it already read are lost",
        experimental: false,
        lists: &[List {
            name: "buffered",
            what: "types that hold bytes read ahead from their source",
            default: &["bufio.Reader", "bufio.ReadWriter"],
        }],
        run: resources::dropped_reader,
    },
    Check {
        name: "error-eq",
        finds: "an error that can wrap compared with == or != to a sentinel error: a wrapped sentinel does not match, errors.Is does",
        experimental: false,
        lists: &[List {
            name: "unwrapped",
            what: "calls whose error may wrap a sentinel any caller chose",
            default: &["context.Cause"],
        }],
        run: values::error_eq,
    },
    Check {
        name: "lock-without-unlock",
        finds: "a Lock or RLock with a path to a return that passes no matching unlock, deferred or not",
        experimental: false,
        lists: &[
            List {
                name: "lock",
                what: "calls that take a lock",
                default: &["sync.Mutex.Lock", "sync.RWMutex.Lock", "sync.RWMutex.RLock"],
            },
            List {
                name: "unlock",
                what: "the call that releases the lock at the same index of lock",
                default: &[
                    "sync.Mutex.Unlock",
                    "sync.RWMutex.Unlock",
                    "sync.RWMutex.RUnlock",
                ],
            },
            List {
                name: "holders",
                what: "words of a function name that make holding the lock on return its purpose",
                default: &["lock", "rlock", "acquire"],
            },
        ],
        run: checks::lock_without_unlock,
    },
    Check {
        name: "timer-not-stopped",
        finds: "a method that stops some timers of its type's fields but not one another method arms",
        experimental: false,
        lists: &[
            List {
                name: "arm",
                what: "calls that make a timer",
                default: &["time.AfterFunc", "time.NewTimer"],
            },
            List {
                name: "stop",
                what: "calls that stop a timer",
                default: &["time.Timer.Stop"],
            },
        ],
        run: resources::timer_not_stopped,
    },
    Check {
        name: "conn-leak",
        finds: "a connection or file a function opens, on a path to an error return that neither closes, returns nor hands it on",
        experimental: false,
        lists: &[
            List {
                name: "resources",
                what: "types of an open connection or file",
                default: &[
                    "net.Conn",
                    "net.TCPConn",
                    "net.UDPConn",
                    "net.UnixConn",
                    "net.IPConn",
                    "net.PacketConn",
                    "net.Listener",
                    "net.TCPListener",
                    "net.UnixListener",
                    "crypto/tls.Conn",
                    "os.File",
                ],
            },
            List {
                name: "close",
                what: "methods that close a resource",
                default: &["Close"],
            },
        ],
        run: resources::conn_leak,
    },
    Check {
        name: "accept-blocking",
        finds: "a function that accepts connections and then makes a blocking read or handshake itself, not in a goroutine, with no deadline set",
        experimental: false,
        lists: &[
            List {
                name: "accept",
                what: "calls that accept a connection",
                default: &[
                    "net.Listener.Accept",
                    "net.TCPListener.Accept",
                    "net.TCPListener.AcceptTCP",
                    "net.UnixListener.Accept",
                    "net.UnixListener.AcceptUnix",
                ],
            },
            List {
                name: "blocking",
                what: "calls that wait for the peer",
                default: &[
                    "io.ReadFull",
                    "io.ReadAll",
                    "io.ReadAtLeast",
                    "net.Conn.Read",
                    "bufio.Reader.Read*",
                    "crypto/tls.Conn.Handshake",
                    "crypto/tls.Conn.HandshakeContext",
                    "crypto/tls.Conn.Read",
                    "net/http.ReadRequest",
                    "github.com/gobwas/ws.Upgrader.Upgrade",
                    "github.com/gobwas/ws.HTTPUpgrader.Upgrade",
                ],
            },
            List {
                name: "deadline",
                what: "calls that bound how long a read waits",
                default: &["SetDeadline", "SetReadDeadline"],
            },
        ],
        run: resources::accept_blocking,
    },
    Check {
        name: "after-in-loop",
        finds: "time.After called in a loop that also resets a timer: the wait starts over on every turn and never ends while the timer fires",
        experimental: false,
        lists: &[
            List {
                name: "after",
                what: "calls that make a one-shot wait",
                default: &["time.After"],
            },
            List {
                name: "periodic",
                what: "calls that rearm a timer the same loop waits on",
                default: &["time.Timer.Reset", "time.Ticker.Reset"],
            },
        ],
        run: resources::after_in_loop,
    },
    Check {
        name: "frame-not-consumed",
        finds: "a frame header read whose payload is neither read nor discarded on a path back to the next header read",
        experimental: false,
        lists: &[
            List {
                name: "open",
                what: "calls that read a frame header and leave its payload to the caller",
                default: &[
                    "github.com/gobwas/ws/wsutil.Reader.NextFrame",
                    "github.com/gobwas/ws.ReadHeader",
                ],
            },
            List {
                name: "consume",
                what: "calls that read or skip a payload",
                default: &[
                    "github.com/gobwas/ws/wsutil.Reader.Discard",
                    "github.com/gobwas/ws/wsutil.Reader.Read",
                    "io.ReadFull",
                    "io.ReadAll",
                    "io.ReadAtLeast",
                    "io.Copy",
                    "io.CopyN",
                ],
            },
        ],
        run: resources::frame_not_consumed,
    },
    Check {
        name: "quadratic-insert",
        finds: "a loop that calls, once per turn, an insert method that scans every entry its receiver holds",
        experimental: false,
        lists: &[List {
            name: "insert-scan",
            what: "methods that add an entry to a container, and may scan it first",
            default: &[
                "Add", "Set", "Put", "Insert", "Append*", "add", "set", "put", "insert",
            ],
        }],
        run: values::quadratic_insert,
    },
    Check {
        name: "unreached",
        finds: "functions no entry reaches, as codespec reach gives them",
        experimental: true,
        lists: &[List {
            name: "generated",
            what: "file names of generated code, which is not reported (cir/1 keeps no generated mark)",
            default: &[
                "*.pb.go",
                "*.pb.gw.go",
                "*_gen.go",
                "*.gen.go",
                "*_generated.go",
                "zz_generated*",
            ],
        }],
        run: checks::unreached,
    },
];

/// The lists a names file replaces, by list name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    lists: BTreeMap<String, Vec<String>>,
}

impl Names {
    /// A names file: a JSON object of list name to array of entries. A name no check has is
    /// refused, naming it.
    pub fn from_json(bytes: &[u8]) -> Result<Names, String> {
        let lists: BTreeMap<String, Vec<String>> = serde_json::from_slice(bytes)
            .map_err(|e| format!("names: an object of list name to entries: {e}"))?;
        for name in lists.keys() {
            if list(name).is_none() {
                let known: Vec<&str> = CHECKS
                    .iter()
                    .flat_map(|c| c.lists.iter().map(|l| l.name))
                    .collect();
                return Err(format!(
                    "names: no list {name}; the lists are {}",
                    known.join(", ")
                ));
            }
        }
        Ok(Names { lists })
    }

    /// The entries of the list `name`: the names file's, or its default.
    pub fn get(&self, name: &str) -> Vec<String> {
        match self.lists.get(name) {
            Some(v) => v.clone(),
            None => list(name)
                .map(|l| l.default.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
        }
    }
}

fn list(name: &str) -> Option<&'static List> {
    CHECKS
        .iter()
        .flat_map(|c| c.lists.iter())
        .find(|l| l.name == name)
}

/// How a check runs.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// `insecure-random`: every call into a random package.
    pub strict: bool,
    pub names: Names,
    /// Keep the findings of the modules whose path (or name) is one of these or lies under one.
    pub report: Vec<String>,
    /// `unreached`: the entry functions, `<path>.<key>`.
    pub entries: Vec<String>,
    /// `unreached`: modules whose every exported function is an entry.
    pub exported: Vec<String>,
}

/// What a check matched, at a position (`<module path>/<file>:<line>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub what: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub pos: String,
    pub function: String,
    pub check: String,
    pub evidence: Vec<Evidence>,
}

impl Finding {
    /// The text output's line: `<pos> <function> <check>: <evidence>`.
    pub fn line(&self) -> String {
        let evidence: Vec<String> = self
            .evidence
            .iter()
            .map(|e| match &e.pos {
                Some(pos) => format!("{} at {pos}", e.what),
                None => e.what.clone(),
            })
            .collect();
        format!(
            "{} {} {}: {}",
            self.pos,
            self.function,
            self.check,
            evidence.join("; ")
        )
    }
}

/// A `codespec-check-report/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub format: String,
    pub check: String,
    pub findings: Vec<Finding>,
}

impl Report {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Report always serializes");
        s.push('\n');
        s
    }

    /// One line per finding.
    pub fn to_text(&self) -> String {
        self.findings.iter().map(|f| f.line() + "\n").collect()
    }
}

/// Every check, its line and its lists as `codespec check --list` prints them, with the lists
/// `names` gives.
pub fn list_text(names: &Names) -> String {
    let mut out = String::new();
    for c in CHECKS {
        let mark = if c.experimental {
            " (experimental)"
        } else {
            ""
        };
        out += &format!("{}{mark}  {}\n", c.name, c.finds);
        for l in c.lists {
            out += &format!("    {}: {}\n", l.name, names.get(l.name).join(" "));
        }
    }
    out
}

/// Run the check `check` over `modules`, read as one program. An unknown check, a `lock` list
/// whose length differs from `unlock`'s, or an entry naming no function of the set is refused.
pub fn run(check: &str, modules: &[Module], opts: &Options) -> Result<Report, String> {
    let Some(c) = CHECKS.iter().find(|c| c.name == check) else {
        let names: Vec<&str> = CHECKS.iter().map(|c| c.name).collect();
        return Err(format!(
            "no check {check}; the checks are {}",
            names.join(", ")
        ));
    };
    let set = set::Set::new(modules);
    let ctx = checks::Ctx {
        set: &set,
        opts,
        check: c.name,
    };
    let mut findings = (c.run)(&ctx)?;
    findings.sort_by(|a, b| {
        position_key(&a.pos)
            .cmp(&position_key(&b.pos))
            .then_with(|| a.function.cmp(&b.function))
            .then_with(|| a.evidence.len().cmp(&b.evidence.len()))
    });
    findings.dedup();
    Ok(Report {
        format: FORMAT.to_string(),
        check: c.name.to_string(),
        findings,
    })
}

/// `file:line` as `(file, line)`, so `m.go:9` sorts before `m.go:10`.
fn position_key(pos: &str) -> (&str, u64) {
    match pos.rsplit_once(':') {
        Some((file, line)) => (file, line.parse().unwrap_or(0)),
        None => (pos, 0),
    }
}

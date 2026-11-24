//! The Rust reader: one crate to one `cir/1` module, parsed with `syn`, every name and type
//! resolved by the reader itself for the subset of Rust `cir/1` expresses. The crate is its root
//! file (`[lib] path`, `src/lib.rs`, or `src/main.rs` for a crate without a library) and every
//! module file its `mod` declarations name, the items of all its modules read as one list
//! (`files`); a position names the file it stands in, relative to the root file's directory
//! (`lib.rs:12`, `wire.rs:40`, `net/mod.rs:3`). Nothing is guessed: a construct the reader cannot
//! type or map is kept as a `foreign` statement in `rust` (its source, the reason and the calls it
//! makes), with a refusal at its position; an item it cannot map (a function with a bound `cir/1`
//! has no constraint for, a trait outside the subset, a type outside it) is listed as a refusal
//! and left out, and a function among them is an `opaque` function with the calls of its body
//! (`calls`).
//!
//! `cir/1` is Go-shaped: the passes and writers expect `(T, error)` results, `(T, bool)` comma-ok
//! results, pointer receivers, sentinel errors and channels of `any`. The reader maps Rust's idioms
//! back to those shapes; it is the inverse of the Rust writer (`adapters/rust/src/write.rs`).
//!
//! # Items
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `struct T { f: U, … }` | `struct`; a `pub` field is exported (`id` → `ID`), another unexported (`seq`) |
//! | `struct T(pub U)`, `U` an integer, `bool` or `String` | `newtype` |
//! | `enum T { A, B }` (unit variants) | `enum` over `int` (platform), values the discriminants (`A = 1`) or 0, 1, … |
//! | the same with `fn value(&self) -> &'static str` (or an integer) matching every variant to a literal | `enum` over `string` (or that integer), the method absorbed |
//! | `enum T { A { x: U }, B(U) }` | `union`, tuple fields named `_0`, `_1`, …: the `_<digits>` rule, such a name marks a positional field, so the Rust writer writes `B(U)` back; a struct variant whose field is named `_<digits>` (`V { _0: U }`) would read the same and is refused at that field |
//! | `enum Error` of unit variants, `V(S)` (`S` a struct) and `Msg(String)`, with a `Display` impl | sentinel errors `Err<V>` (message from its `Display` arm); `S` an error struct whose `Display` is its `Error()`; `Msg` `errorf` |
//! | `enum M { A(A), B(B) }`, each variant wrapping the struct of its name | absorbed: `Sender<M>` is `chan<- any` and `M::A(x)` is `x` |
//! | `impl fmt::Display for T` | a value-receiver method `String() string` (`Error() string` for an error struct) returning what it writes; absorbed when it only calls such a method, `self.value()` of a string enum, or the inner value of a string newtype. When the impl is refused, each `{}` and `to_string()` of the type is refused at its line |
//! | `impl std::error::Error for …` (empty) | absorbed |
//! | `#[serde(…)]` on a struct, enum, variant or field with any key but `bound` and `crate` (`rename`, `rename_all`, `skip`, `skip_serializing_if`, `flatten`, `default`, `tag`, `untagged`, `with`, `transparent`, …), or an attribute of a serde helper crate (`#[serde_as]`) | the type refused, the reason naming the attribute: `cir/1` carries no wire names or shape, and the type read without them would serialize differently |
//! | `pub fn f`, `fn f` | `F`, `f` (Go spelling, golint's initialisms). Two functions (associated ones among them), two methods of one type, two constants or two statics of one Go spelling (`value_2`, `value2`) are each refused at their line and each use of them refused (a crate trait's impl with such a method is refused whole); so are two fields of a struct (refusing it), two parameters of a function (refusing it) and two methods of a trait (refusing it). Items of different kinds are apart in `cir/1` and keep their names; locals are renamed apart, and a `let` of a function's or closure's outermost block that shadows a parameter binds a new name, as Go's parameters share that block's scope |
//! | `impl T { fn m(&mut self) }` | method with receiver `*T`; once one method of `T` takes `&mut self`, every `&self` method of `T` has `*T` too, otherwise `&self` and `self` are `T` |
//! | `impl T { fn f(…) }` without `self` | function `F<T>` (`new` → `NewT`) |
//! | `const X: T = <literal>` | `const`; one whose type or value is refused is refused at its line, not declared, and each use of it refused naming it |
//! | `static X: T = <literal>;`, `static mut X: T = <literal>;` | `var` (`X` → `X`, `HITS` unexported → `hits`); a zero literal is a `var` without a value. A static whose initialiser is not a literal is refused at its line, not declared, and each use of it refused naming it |
//! | `#[derive(Default)] struct T` whose every field's default is Go's zero value (an integer, float, `bool`, `String`, `Vec`, map, such a struct, an array of these) | `T::default()` is `zero` of `T`; another `T::default()` is `foreign` |
//! | `trait Tr { fn m(&self, …) -> R; }` | `interface`, its methods sorted by name; a generic or `unsafe` trait, a supertrait, an item other than a method, a default body, a receiver other than `&self` or a result other than a plain value refuses the trait and every `dyn Tr` |
//! | `impl Tr for T`, `Tr` a trait of the crate | the methods of `T`, exported as the trait is; refused at the impl when an inherent impl of `T` or another crate trait's impl for `T` has a method of the same name (Go has one method per name), or when `T` is not a type the crate declares (`impl Tr for i64`, `impl Tr for &T`). A generic call, a conversion to `dyn Tr` or a call of a method of `Tr` on a `T` the crate also implements `Tr` for `&T` or `&mut T` is refused, as the reader reads `&T` as `T` |
//! | `fn f<T, U: Tr>(…)`, `fn f<T>(…) where T: Tr` | a function with `tparams`: `T` constrained by `any`, `U` by the interface `Tr`, a trait of the crate |
//! | `struct S<T> { … }` | a `struct` with `tparams`, its fields of type `param` |
//! | `impl<T> S<T> { fn m(&self) }` | a method with receiver `S[T]`, binding `T`; a function without `self` there is generic over `T` too. The impl names each of the struct's parameters once, in order, with the struct's bounds |
//! | a bound other than none or one trait of the crate (`Clone`, `Copy`, `PartialEq`, `Ord`, `Sized`, two bounds, a lifetime), a `where` clause other than `T: Tr`, a lifetime or const parameter, an associated type (`T::Item`, `Iterator<Item = i64>`), `impl Tr` as a parameter type, a method with its own type parameters (Go's methods have none), a generic trait, enum or tuple struct, an impl of a trait for a generic struct | refused: `cir/1` constrains a type parameter by `any` or one interface, and has no lifetimes, const parameters or associated types |
//! | a raw identifier (`r#type`) | its name without `r#` everywhere, a keyword with `_` added (`type_`, so `Type` or `type_` in Go), or `type_1`, `type_2`, … when another name of the file (a raw `r#type_` among them) would read the same; a type or trait the crate declares under a raw keyword is named by its visibility (`pub struct r#match` → `Match`, `struct r#match` → `match_`, `enum r#type` → `type_`), `_` added while that is a Rust keyword or another name of the file; under a raw name that is no keyword (`r#Foo`) it is that name, as a plain `Foo` writes it |
//! | `use` | nothing of its own: a path names the full path its module's `use` declarations make of it. A std type, trait or function of the reader's tables is matched by that full path, never by its last segment: `Sender` after `use std::sync::mpsc::Sender;` or `std::sync::mpsc::Sender` is a channel, `broadcast::Sender` after `use tokio::sync::broadcast;` is refused. Bare, only the prelude's `String`, `Vec`, `Box`, `Option`, `Result` and `Default` are std's; `HashMap` needs its `use` (a glob import or a `use` inside a function body is not read, so such a name is refused). A name a `use` takes from outside the crate is not the crate's item of that name, and is refused. The crate's own items resolve by name (`files`), and a call `g(…)` of a function the module imports as `use crate::m::f as g;` is a call of `f`. The calls an `opaque` function or a `foreign` statement lists resolve a path the same way (`calls`) |
//! | `mod m;`, `mod m { … }` | the items of the module, read with the crate's other items; a module, item or part of one built only for tests (`#[cfg(test)]`) is not read, and one under a `cfg` the reader cannot resolve (a feature, a platform) is refused (`files`). Two types, traits, functions, or constants and statics of one name in two modules are each refused, with every impl of such a type, as a use could name either (`files`) |
//!
//! # Types
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `i8`…`i64`, `u8`…`u64` | `int` of that width; `isize`, `usize` `int` (platform), signed and unsigned; a length and an index are `usize` |
//! | `bool`, `String`, `&str` | `bool`, `string` |
//! | `f32`, `f64` | `float` of that width |
//! | `[T; N]`, `N` a literal | `array` |
//! | `fn(A, B) -> R` | `func` |
//! | `Rc<dyn Any>` (`std::any::Any`) | `any` |
//! | `Rc<(SyncSender<T>, Receiver<T>)>` | `chan T`, both ends: the Rust writer's form of a channel one function sends on and receives from |
//! | `dyn Tr`, `Box<dyn Tr>`, `Tr` a trait of the crate | the interface `Tr` |
//! | `Vec<T>`, `HashMap<K, V>`, `BTreeMap<K, V>` | `slice`, `map` |
//! | `Rc<RefCell<T>>` | `ptr` to `T` |
//! | a local bound to `&mut x`, `x` a local | `ptr` to the type of `x` |
//! | `Sender<T>` | `chan<- T`; `chan<- any` for an absorbed message enum |
//! | `&T` | `T`: a shared reference is read as the value it refers to, which Rust's borrow rules keep unchanged while it lives |
//! | result `Result<T, Error>` | `(T, error)`; `Result<(), Error>` is `error`, `Result<(A, B), Error>` `(A, B, error)` |
//! | result `Option<T>` | `(T, bool)` |
//! | result `(A, B)` | `(A, B)` |
//! | `T`, a type parameter in scope | `param` |
//! | `S<A>`, `S` a generic struct | `named` `S` with type arguments `A` |
//! | other `dyn` and `impl` traits, `&mut T` elsewhere, other `Box`es, other paths | refused |
//! | a path the table names by its last segment only (`tokio::sync::broadcast::Sender<T>`, `other::Vec<T>`, `Rc<other::RefCell<T>>`) | refused: each row above is matched by the full path the type names after `use` (see Items) |
//!
//! # Statements
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `#[cfg(test)] s`, `#[cfg(all(test, …))] s`; `#[cfg(not(test))] s` | nothing, as a normal build has none; `s`. A statement under a condition the reader cannot resolve (`#[cfg(feature = "x")]`, `#[cfg(debug_assertions)]`) is `foreign` (`files`) |
//! | `let x = e;` | `let` |
//! | `let mut x: T = <zero of T>;` (`0`, `0.0`, `false`, `String::new()`, `Vec::new()`, `[<zero>; N]`) | `var x T` |
//! | `let x: Rc<dyn Any> = e;` | `var x any`, then `x = e`, so that `x` holds `any` and not the type of `e` |
//! | `let mut ks: Vec<K> = m.keys().copied().collect(); ks.sort();` (`cloned`, `sort_unstable`; `String` keys `ks.sort_by(\|a, b\| a.encode_utf16().cmp(b.encode_utf16()))`) | `ks := sorted_keys(m)`; `String` keys sorted by byte are `foreign`, since `sorted_keys` orders by UTF-16 code unit |
//! | `unsafe { … }` around a `static mut` of the crate | its statements; an `unsafe` block that binds a name, or that holds anything a refusal keeps, is `foreign` |
//! | `let Some(x) = m.get(&k)[.cloned()/.copied()] else { …; return … };` | `lookup_else` |
//! | `let Some(x) = f(…) else { … };`, `f` returning `Option` | `x, ok := f(…)`, `if !ok { … }` |
//! | `let p = &mut x;`, `x` a local | `p := &x`; `*p` is then `*p`, read and assigned through |
//! | `x = e;`, `x op= e;` | `assign`, `op` kept; `x &= !y` on integers is `x &^= y`. Rust evaluates `e` before the place's operands (`v[i()] = f()` calls `f` first), and for `op=` on integers, floats and `bool` before it loads the place; `cir/1` the place first. Where either side runs code (a call of the crate's or a closure's, a method outside a fixed std set, a macro, a block), `e` is bound to a new local first and the assignment takes that local; a place that is a variable or a static needs none for `=`. `op=` on a `String` (`AddAssign`, place first, as `cir/1`) whose place goes through a static or a method call while `e` runs code is refused, and so is `op=` on any other type (an operator impl the reader does not read) |
//! | `(a, b) = (x, y);` | one `assign` of every value to every target (Rust evaluates the values first, as Go does), under the conditions of the Rust writer's value block |
//! | `x = x.wrapping_add(1)`, `x = x.wrapping_sub(1)` | `incr`, `decr` |
//! | `x = x.wrapping_<op>(e)` | `assign` with `<op>=` |
//! | `m.insert(k, v);`, `v.push(x);`, `s.push_str(t);` | `m[k] = v`, `v = append(v, x)`, `s += t`; a `push` whose place runs code (it would run twice), and any of them whose place goes through a static or a method call while an argument runs code, are refused |
//! | `ch.send(x).expect(…)` / `.unwrap()` | `send` |
//! | `ch.0.try_send(x).expect(…)` / `.unwrap()`, `ch` a channel of both ends | `send` (a full channel panics, where Go would block) |
//! | `if`, `else if`, `else` | `if` |
//! | `for (k, v) in m` / `&m` / `m.iter()` / `m.clone()`; `m.keys()`, `m.values()`; `for x in v` (same forms); `for (_, x) in v.iter().enumerate()` | `for_range` over the map or slice; a named index of `enumerate()` is refused, as it is a `usize` and the range gives a signed platform `int`. A `HashMap` visits its keys in no order Rust specifies, as `cir/1`'s map range does. A `BTreeMap` visits them in order: with integer keys it is `for _, k := range sorted_keys(m)`, each value `m[k]` (a source a call gives, or a `clone()` whose values are read, is refused); with other keys it is refused, as Rust orders `String` keys by byte and `sorted_keys` by UTF-16 code unit. The map's type is told from the local's or parameter's written type or `BTreeMap::new()`/`HashMap::new()`, or the field's declared type; a map whose type is not told counts as a `BTreeMap` when the crate names `BTreeMap` or `BTreeSet`, as a `HashMap` otherwise |
//! | `while c { … }`, `loop { … }` | `for` with and without a condition; a last statement that steps the loop (an assignment) is its `post` when every `continue` runs it first (`if …{ post; continue; }`, or the writer's `{ post; continue; }`) |
//! | `break`, `continue` (without a label) | `break`, `continue`; a `break` in a `match` arm leaves the loop, as `cir/1`'s `break` in a `switch` does |
//! | `match x { A => …, B | C => …, _ => … }` on literals and unit variants | `switch`; an exhaustive match on an enum's variants makes its last arm the default |
//! | an arm `v if v == e => …`, `v` read by neither `e` nor the body | a `case e:` of that `switch`; any other guard is refused |
//! | `let v = f(…)?;`, `let (a, b) = f(…)?;`, `f(…)?;`, `f` a fallible function or method of the crate, in a function whose result is a `Result` | `v, err := f(…)` (`_, err`, or `err` alone for a `Result<(), Error>`), then `if err != nil { return <zero values>, err }`; `err` a name no other binding holds. A `?` on an `Option`, on a call outside the crate, inside an expression or in a function whose result is not a `Result` is refused |
//! | `return …`, a block's tail expression | `return`, with `Ok(x)` → `x, nil`, `Err(e)` → zero values and `e`, `Some(x)` → `x, true`, `None` → `zero, false` |
//! | `println!("{} {}", a, b)` (only `{}` separated by single spaces) | `println` |
//!
//! Anything else (`while let`, labels, ranges, closures other than below, `unsafe` other than
//! above, `?` other than above, nested items, blocks other than the Rust writer's, macros other
//! than those named) is `foreign`.
//!
//! # Expressions
//!
//! Every expression's type comes from a local's binding, a parameter, a field declaration, a
//! signature, a literal's suffix or the type the context expects (an annotation, the other operand,
//! a field, a parameter, a result). An unsuffixed integer or float literal with no such context is
//! refused, not defaulted to `i32` or `f64`.
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `1i64`, `"s"`, `true` | `lit` in Go's spelling (`"s"` quoted as Go quotes it) |
//! | `1.5f64`, `0.25` where a float is expected | `lit` of that `float`, spelled as `cir/1` spells the value at its width |
//! | `T::V` (unit variant), `CONST` | `const` |
//! | `X`, `unsafe { X }`, `X` a `static` | `global` |
//! | `f`, a function of the crate as a value | `func_ref`; a generic function as a value is refused, as `cir/1`'s `func_ref` names no instantiation |
//! | `f::<A>(x)`, `S::<A>::g(x)`, `S::<A> { … }` | `call` or `struct` with type arguments `A` |
//! | `f(x)`, `S { … }` of a generic function or struct | the same, its type arguments inferred from the arguments' or fields' types and then the type expected, as Rust infers them, and written out; one nothing fixes is refused. Each satisfies its constraint in Go: a type whose methods have pointer receivers there does not |
//! | `x.m(…)`, `x` of a type parameter bound by `Tr` | `iface_call` through `Tr` |
//! | `f(x)`, `f` a local of a `fn` type; `(s.f)(x)` | `call_value` |
//! | `\|x: T\| -> R { … }` | `lambda`; its captures are the locals of the enclosing function it reads or writes, sorted. A `move` closure (it copies what it captures, where Go shares it), an untyped parameter or result, or a body other than a block is `foreign` |
//! | `x.m(…)`, `x` of a crate trait's `dyn` | `iface_call` |
//! | a value of a crate type where its trait is expected (a parameter, field, result, `Box::new(x)`) | `to_iface`; a type that does not implement it is refused |
//! | `T::default()`, see Items | `zero` |
//! | `Rc::new(x) as Rc<dyn Any>`, `Rc::new(x)` where `any` is expected | `x` |
//! | `ch.1.try_recv().expect(…)` / `.unwrap()`, `ch` a channel of both ends | `<-ch` (an empty channel panics, where Go would block); `recv()` is `foreign` |
//! | `Error::V`, `Error::S(s)`, `Error::Msg(format!(…))` | `errvar`, `to_error`, `errorf` |
//! | `T(x)` (newtype) | `conv` |
//! | `T { f: x, .. }` | `struct`; a field whose value is the zero literal (`0`, `false`, `""`, `String::new()`) is left out, as Go does |
//! | `Rc::new(RefCell::new(x))` | `&x` |
//! | `format!("a {} {:>3}", x, y)` | `sprintf("a %s %3d", x, y)`: `{}` is `%d` for an integer, `%t` for a `bool`, `%s` otherwise; `{x}` captures the local; `>N`, `<N`, `0N` widths |
//! | `+ - * / % & | ^`, `<< >>` (next row), comparisons, `&& || !`, unary `-` | `binop`, `unop` (Rust's `+` panics on overflow where Go wraps: a trace that overflows is not equal by construction); on floats only `+ - * /` and comparisons, since Go has no `%` on them; functions do not compare |
//! | `x << n`, `x >> n`, `x <<= n`, `x >>= n` on integers | `binop`, `assign`, only where the count `n` is proven below the width W of `x`'s type (8, 16, 32, 64): an integer literal below W (unsuffixed, Rust's `i32`), a `const` whose literal is below W, `e & m` with a literal `m` at most W - 1, or `e % k` with a literal `k` at most W and `e` unsigned; on a `usize` or `isize` (`platform`, 32 or 64 bits) only an integer literal below 32. Any other shift is refused at its line: Rust panics on a count at or past W with `overflow-checks` and masks it without, where Go gives 0 or the sign |
//! | `==`, `!=` (and a match guard `v if v == e`) on a type whose `PartialEq` the crate writes by hand; `<`, `<=`, `>`, `>=` on one whose `PartialOrd` or `Ord` it does; either on a type that holds such a type (a field, the type a newtype wraps, a type argument, an element) | refused at its line: Rust calls the impl, where `cir/1`'s comparison is Go's built-in one. A derived `PartialEq`, `PartialOrd` or `Ord` reads as the built-in comparison |
//! | `x & !y` on integers | `x &^ y`, as the Rust writer writes Go's bit clear |
//! | `x.clone()` of a value without a slice or map inside, an array's elements and a generic struct's type arguments included | `x` (Go copies) |
//! | `*p`, `p` bound to `&mut x` | `*p` |
//! | `&x`, `*x` of a non-pointer | `x` |
//!
//! ## Standard methods
//!
//! The fixed table of std methods the reader types. A method outside it, on a std type, is refused.
//! A function of a type (`String::from`, `HashMap::new`, `Rc::new`, `RefCell::new`, `Box::new`) is
//! the table's by the full path of its type, as Types matches it; the macros `format!`,
//! `println!`, `vec!` and `write!` are std's bare, unless the crate defines or a `use` brings in
//! one of that name, or as `std::`, `core::` or `alloc::` paths.
//!
//! | receiver | method | cir/1 |
//! |---|---|---|
//! | `String` | `clone`, `to_string`, `to_owned`, `as_str`, `into` | the value |
//! | `String`, `Vec`, `HashMap`, `BTreeMap` | `len()`; `len() as i64` | `len`: unsigned platform `int` (`usize`); `int 64` for `as i64` |
//! | `String`, `Vec`, `HashMap`, `BTreeMap` | `is_empty()` | `len(x) == 0` |
//! | `String` | `push_str(t)` (statement) | `s += t` |
//! | `String::from(x)`, `String::new()` | | `x`, `""` |
//! | `Vec` | `push(x)` (statement) | `v = append(v, x)` |
//! | `Vec::new()`, `HashMap::new()`, `BTreeMap::new()`, `vec![]` | | `make` of the expected type |
//! | `HashMap`, `BTreeMap` | `insert(k, v)` (statement) | `m[k] = v` |
//! | `HashMap`, `BTreeMap` | `get(&k)` in `let Some(x) = … else` | `lookup_else` |
//! | `HashMap`, `BTreeMap` | `get(&k).cloned()` / `.copied()` then `.unwrap_or(<zero>)` / `.unwrap_or_default()` | `m[k]` |
//! | `HashMap`, `BTreeMap`, `Vec` | `iter()`, `keys()`, `values()`, `clone()` as a `for` source | the map or slice (a `BTreeMap` in key order, see Statements) |
//! | `Option` (from `get`) | `cloned()`, `copied()` | the same lookup |
//! | `Rc<RefCell<T>>` | `borrow()`, `borrow_mut()` | the `T` behind the pointer: its fields are the pointer's, `.clone()` of it is `*p` |
//! | `Rc<RefCell<T>>` | `clone()`, `Rc::clone(&p)` | the pointer |
//! | `Sender<T>` | `send(x).expect(…)`, `send(x).unwrap()` (statement) | `send` |
//! | integers | `wrapping_add`, `wrapping_sub`, `wrapping_mul`, `wrapping_div`, `wrapping_rem` | `+ - * / %` |
//! | integers, floats, `bool`, `Rc<dyn Any>`, `fn` | `clone()` | the value |
//! | `Rc<(SyncSender<T>, Receiver<T>)>` | `.0.try_send(x)`, `.1.try_recv()`, each then `expect(…)` / `unwrap()` | `send`, `<-ch` |
//! | a type with `Display`, integers | `to_string()` | `sprintf("%s", x)`, `sprintf("%d", x)` |
//!
//! # What the reader claims, and what it does not
//!
//! `codespec-read-rust --describe` claims each reader feature (the vocabulary without `fact.*`)
//! whose conformance case has a hand-written Rust source, `conformance/cases/<case>/rust/case.rs`.
//! `tests/cases.rs` holds every claim: the source runs as the Go reference does, reads without a
//! refusal and with its subject, and passed and written as Go runs as the reference does; a module
//! the Go writer refuses for a subject it does not claim (`decl.union`, Go has no tagged union) the
//! evaluator runs instead, as the reader check's step 4 does. `stmt.foreign` is held the same way:
//! its source keeps a labelled loop as a `foreign` statement with a refusal, on a path the calls
//! do not take, and the evaluator runs the module the Go writer refuses. It claims 113 of the 121;
//! these stay unclaimed. Each reason says whether it holds of Rust or of this reader only:
//!
//! | feature | why |
//! |---|---|
//! | `fact.*` | facts are computed by `codespec pass`, never read |
//! | `unop.plus` | Rust: the language has no unary `+` |
//! | `stmt.raise` | Rust: a `panic!` is no value of another crate's type handed to a framework, and a `Result` is a result; the case's module is written by hand |
//! | `decl.embedded` | Rust has no struct embedding: every field is named. Its nearest form, `impl Deref<Target = Inner> for Outer`, lets one field's fields and methods be reached through the outer value by auto-deref; this reader refuses an impl of a std trait other than `Display` and so does not read it. Reading it as `embedded` would take mapping that impl and writing each access it makes possible as the explicit path `cir/1` records |
//! | `decl.variadic`, `expr.spread` | Rust: a function takes a fixed number of arguments (C-variadic `...` exists only on `extern "C"` declarations, and a variadic-looking `println!` is a macro); a slice parameter is a `slice` |
//! | `expr.extern`, `expr.extern_method`, `type.extern` | this reader, not Rust: Rust calls other crates and its std (`s.repeat(n)`, `String::with_capacity`), and the reader refuses each call, method and type outside its std table instead of reading it as an extern. Claiming them takes (1) reading such a call as `extern`/`extern_method` with an `externs` entry of its path and signature, which the reader would take from the call's argument and result types, as it reads one file without the crate's dependencies, and (2) step 4 running the module: the Go program written from it calls the extern's path as a Go package, so a Rust path (`std::…`) would need a Go package of that path doing what Rust's does. The cases' references call Go's `strings` and `strconv`, which a Rust source does not name |//!
//!
//! # Spans
//!
//! [`read_crate_spans`] (`codespec-read-rust --spans <dir>`) reads the same module and tells
//! where its functions, struct fields and named nodes stand in the crate's files, and which `use`
//! paths name its items: the lists of a `codespec-spans/1` document. What each span covers, and
//! when a node is `lowered`, is the module `spans`.
//!
//! # Limits
//!
//! A source is untrusted: what it can cost the reader is bounded, and a source past a bound is
//! refused, exit 1 and the reason on stderr, not a crash.
//!
//! | bound | limit |
//! |---|---|
//! | size | `Cargo.toml` and each source file up to [`MAX_SOURCE_BYTES`], 4 MiB each, read no further than one byte past it; a module file past it is a refusal at its `mod` declaration |
//! | files | [`MAX_FILES`], 4096 files, and [`MAX_CRATE_BYTES`], 64 MiB of source over all of them: a crate past either is refused |
//! | module depth | [`MAX_MODULE_DEPTH`], 64 modules deep (`crate::a::b` is 2): a `mod` declaring a deeper module is refused at its line |
//! | nesting | [`MAX_NESTING`], 256 levels per file, counted on the source's tokens before it is parsed (`lib.rs:<line>: nested deeper than 256 levels, this reader's limit`; a module file past it is a refusal at its `mod` declaration). A level is a delimited group, a keyword or a punctuation character but `:`, `,` and `'` (`&&` is two, as `!!x` is), counted from the start of its group, a `;`, a `=>` or a block that ends a statement or an item (an identifier other than `else` or `as`, an attribute or doc comment, a label or a literal follows it); a `,` goes back to the count at the last unclosed `<` or `|`. That counts every level `syn` and the reader recurse on: parentheses, blocks, unary and binary operators, `else if`, method chains, `return`, generic arguments, closures |
//! | stack | parsing, lifting and dropping the syntax tree run on a thread with a 256 MiB stack |

mod body;
mod calls;
mod files;
mod names;
mod spans;

pub use files::{MAX_CRATE_BYTES, MAX_FILES, MAX_MODULE_DEPTH};

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use codespec_model::{
    ConstDecl, ErrorVar, Expr, Field, FloatBits, IfaceMethod, IntWidth, Interface, Module,
    Reader as ReaderInfo, Refusal, Type, TypeDecl, TypeParam, UnionVariant, VarDecl, Variant,
};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;

/// This reader's version, recorded in every module it reads.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Read the crate in `dir`: its library's root file (`[lib] path`, else `src/lib.rs`), or
/// `src/main.rs` for a crate without a library, and every module file it names (`files`). The
/// module is named after the crate's `[package] name`. `Cargo.toml` and the root file are read
/// only as regular files inside `dir`; a manifest that does not parse is an error naming its
/// line, without quoting it.
pub fn read_crate(dir: &Path) -> Result<Module, String> {
    let (name, root) = crate_root(dir)?;
    on_read_thread(|| read_tree(&name, files::crate_tree(dir, &root)?))
}

/// What `codespec-read-rust --spans <dir>` prints: the crate in `dir` read as [`read_crate`]
/// reads it, as one JSON object `{"cir", "files", "spans", "unspanned", "refs"}`
/// (`docs/code-edit-language.md` § 6.3; what each span covers: `spans`), file paths relative to
/// `dir`. The `cir/1` module is the one [`read_crate`] gives.
pub fn read_crate_spans(dir: &Path) -> Result<String, String> {
    let (name, root) = crate_root(dir)?;
    on_read_thread(|| read_tree_spans(&name, files::crate_tree(dir, &root)?, true))
        .map(|(_, out)| out.unwrap_or_default())
}

/// The crate's name (`[package] name`, `-` as `_`) and its root file.
fn crate_root(dir: &Path) -> Result<(String, std::path::PathBuf), String> {
    let manifest = dir.join("Cargo.toml");
    files::regular_inside(&manifest, dir, "the manifest")?;
    let text = read_bounded(&manifest)?;
    let table: toml::Table = toml::from_str(&text).map_err(|e| {
        let at = e
            .span()
            .map(|s| {
                let line = text[..s.start.min(text.len())].matches('\n').count() + 1;
                format!(" at line {line}")
            })
            .unwrap_or_default();
        format!(
            "{}: not a valid manifest{at}: {}",
            manifest.display(),
            one_line(e.message())
        )
    })?;
    let name = table
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| format!("{}: no [package] name", manifest.display()))?
        .replace('-', "_");
    let lib = table
        .get("lib")
        .and_then(|l| l.get("path"))
        .and_then(|p| p.as_str())
        .map(|p| dir.join(p));
    let root = match lib {
        Some(p) => p,
        None if dir.join("src/lib.rs").is_file() || !dir.join("src/main.rs").is_file() => {
            dir.join("src/lib.rs")
        }
        None => dir.join("src/main.rs"),
    };
    Ok((name, root))
}

/// Read one source file as the module `name`, the root of a crate of one file: its inline
/// modules are read, a `mod` naming a file is refused. Positions are `<file>:<line>`. A source
/// nested deeper than [`MAX_NESTING`] is refused before it is parsed.
pub fn read_source(name: &str, file: &str, src: &str) -> Result<Module, String> {
    on_read_thread(|| read_tree(name, files::source_tree(file, src)?))
}

/// `f` run on a thread with a stack of [`STACK_BYTES`]: parsing, lifting and dropping the syntax
/// trees. Spans name their file per thread, so every file of a crate is parsed on this one.
fn on_read_thread<T: Send>(f: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .name("rust-read".into())
            .stack_size(STACK_BYTES)
            .spawn_scoped(s, f)
            .map_err(|e| format!("cannot start a thread to read on: {e}"))?
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// The module of a crate's files: their items read as those of one file.
fn read_tree(name: &str, tree: files::Tree) -> Result<Module, String> {
    read_tree_spans(name, tree, false).map(|(m, _)| m)
}

/// [`read_tree`], and with `spans` what `--spans` prints.
fn read_tree_spans(
    name: &str,
    tree: files::Tree,
    spans: bool,
) -> Result<(Module, Option<String>), String> {
    let files::Tree {
        attrs,
        items,
        sources,
        files,
        span_files,
        refusals,
        apart,
        paths,
        offsets,
    } = tree;
    let recorder = spans.then(|| {
        let order: BTreeMap<&String, usize> =
            files.iter().enumerate().map(|(k, f)| (f, k)).collect();
        let keys = span_files
            .iter()
            .filter_map(|(key, rel)| Some((key.clone(), *order.get(rel)?)))
            .collect();
        let srcs = paths
            .iter()
            .zip(&files)
            .zip(&sources)
            .zip(&offsets)
            .map(|(((path, pos_name), text), offset)| spans::Src {
                path: path.clone(),
                pos_name: pos_name.clone(),
                text: text.clone(),
                offset: *offset,
            })
            .collect();
        spans::Recorder::on(srcs, keys)
    });
    let modules: Vec<String> = items.iter().map(|(_, m)| m.clone()).collect();
    let mut ast = syn::File {
        shebang: None,
        frontmatter: None,
        attrs,
        items: items.into_iter().map(|(item, _)| item).collect(),
    };
    Unraw::of(&ast, &sources).visit_file_mut(&mut ast);
    let tables = call_tables(&ast, &modules);
    let mut r = Reader::new(files.first().map(String::as_str).unwrap_or("lib.rs"));
    r.ordered_maps = sources.iter().any(|src| {
        src.parse::<proc_macro2::TokenStream>().is_ok_and(|ts| {
            let mut names = Taken::default();
            Unraw::plain_idents(ts, &mut names);
            names.names.contains("BTreeMap") || names.names.contains("BTreeSet")
        })
    });
    r.span_files = span_files;
    r.file_order = files.into_iter().enumerate().map(|(k, f)| (f, k)).collect();
    r.refusals = refusals;
    r.tables = tables;
    r.item_modules = modules;
    r.apart = apart;
    if let Some(rec) = recorder {
        *r.spans.borrow_mut() = rec;
    }
    r.scan(&ast);
    let shared = r.spans.clone();
    let rust_names = spans.then(|| spans_names(&r));
    let m = r.module(name, &ast);
    let out = rust_names.map(|names| {
        let rec = std::mem::take(&mut *shared.borrow_mut());
        spans::output(&m, name, rec, &names)
    });
    Ok((m, out))
}

/// How a `use` leaf's item is named in `cir/1`: free functions (read, or opaque as the lister
/// names them) and declared types.
fn spans_names(r: &Reader) -> spans::RustNames {
    let mut n = spans::RustNames::default();
    for (rust, go) in &r.tables.fns {
        n.fns.insert(rust.clone(), go.clone());
    }
    for ((ty, rust), sig) in &r.sigs {
        if ty.is_none() {
            n.fns.insert(rust.clone(), sig.go.clone());
        }
    }
    for t in r.decls.keys() {
        n.types.insert(t.clone(), t.clone());
    }
    n
}

/// The name of the type an impl is of: the last segment of its path, through references.
fn impl_type_name(t: &syn::Type) -> Option<String> {
    match t {
        syn::Type::Path(p) if p.qself.is_none() => {
            p.path.segments.last().map(|s| s.ident.to_string())
        }
        syn::Type::Reference(r) => impl_type_name(&r.elem),
        syn::Type::Paren(p) => impl_type_name(&p.elem),
        _ => None,
    }
}

/// How the crate names each function in `cir/1`, read or opaque alike, with its types, its
/// modules and each module's imports: what [`calls::Lister`] resolves a call with.
fn call_tables(f: &syn::File, modules: &[String]) -> calls::Tables {
    let mut t = calls::Tables::default();
    let mut pub_traits = BTreeSet::new();
    for m in modules {
        let mut p = String::new();
        for seg in m.split("::") {
            if !p.is_empty() {
                p.push_str("::");
            }
            p.push_str(seg);
            t.modules.insert(p.clone());
        }
    }
    for item in &f.items {
        match item {
            syn::Item::Struct(s) => {
                t.types.insert(s.ident.to_string());
            }
            syn::Item::Enum(e) => {
                t.types.insert(e.ident.to_string());
                t.variants
                    .extend(e.variants.iter().map(|v| v.ident.to_string()));
            }
            syn::Item::Union(u) => {
                t.types.insert(u.ident.to_string());
            }
            syn::Item::Type(a) => {
                t.types.insert(a.ident.to_string());
            }
            syn::Item::Trait(tr) => {
                t.types.insert(tr.ident.to_string());
                if is_pub(&tr.vis) {
                    pub_traits.insert(tr.ident.to_string());
                }
            }
            _ => {}
        }
    }
    for (item, module) in f.items.iter().zip(modules) {
        match item {
            syn::Item::Fn(func) => {
                let rust = func.sig.ident.to_string();
                t.fns
                    .insert(rust.clone(), calls::fn_name(&rust, is_pub(&func.vis)));
            }
            syn::Item::Impl(im) => {
                let Some(ty) = impl_type_name(&im.self_ty) else {
                    continue;
                };
                let trait_pub = im.trait_.as_ref().map(|(p, _)| {
                    p.segments
                        .last()
                        .is_some_and(|s| pub_traits.contains(&s.ident.to_string()))
                });
                for ii in &im.items {
                    let syn::ImplItem::Fn(func) = ii else {
                        continue;
                    };
                    let rust = func.sig.ident.to_string();
                    let public = trait_pub.unwrap_or_else(|| is_pub(&func.vis));
                    let has_self = matches!(func.sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
                    let entry = if has_self {
                        (Some(ty.clone()), calls::fn_name(&rust, public))
                    } else {
                        (None, format!("{}{ty}", calls::fn_name(&rust, public)))
                    };
                    t.assoc.entry((ty.clone(), rust)).or_insert(entry);
                }
            }
            syn::Item::Use(u) => t.add_use(module, u),
            syn::Item::Macro(m) if m.mac.path.is_ident("macro_rules") => {
                if let Some(id) = &m.ident {
                    t.macros.insert(id.to_string());
                }
            }
            _ => {}
        }
    }
    t
}

/// The deepest a source may nest, as [`too_deep`] counts it: deeper is refused before parsing.
pub const MAX_NESTING: usize = 256;

/// The largest `Cargo.toml` and source file the reader reads, in bytes.
pub const MAX_SOURCE_BYTES: u64 = 4 << 20;

/// The stack parsing and lifting run on: each recurses a few frames per level, and dropping the
/// syntax tree does too; this holds [`MAX_NESTING`] levels with room to spare.
const STACK_BYTES: usize = 256 << 20;

/// The text of `path`, read up to [`MAX_SOURCE_BYTES`]: a larger file, or one that never ends, is
/// refused.
fn read_bounded(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(format!(
            "{}: over {MAX_SOURCE_BYTES} bytes, this reader's limit",
            path.display()
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| format!("{}: stream did not contain valid UTF-8", path.display()))
}

/// How deep `src` nests as this reader counts it, and the line where it is deepest (0 and 0 for a
/// source that does not tokenize, which `syn` refuses without recursing). It is counted on the
/// tokens `syn` would parse: the text without a byte order mark, and without a first line `#!…`
/// that is no inner attribute; both texts are counted, so the count never misses what `syn`
/// parses. A source deeper than [`MAX_NESTING`] is refused.
pub fn nesting(src: &str) -> (usize, usize) {
    let text = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut texts = vec![text];
    if text.starts_with("#!")
        && let Some(nl) = text.find('\n')
    {
        texts.push(&text[nl..]);
    }
    texts
        .into_iter()
        .filter_map(|t| t.parse::<proc_macro2::TokenStream>().ok())
        .map(deepest)
        .max()
        .unwrap_or((0, 0))
}

/// The deepest level of `ts` and the line of the first token at it.
///
/// A level is each delimited group (`(…)`, `[…]`, `{…}`), each keyword and each punctuation
/// character but `:`, `,` and `'` (`&&` is two, as `!!x` is), counted from the start of the group,
/// a `;`, a `=>`, or a block that ends a statement or an item: one an identifier other than `else`
/// or `as`, a `#` (an attribute or a doc comment of the next item), a `'` (a label) or a literal
/// follows, none of which goes on an expression. A `,` goes back to the count at the last unclosed
/// `<` or the last `|`, so generic arguments and closures nested in one another keep counting. An
/// attribute's `#[…]` is no level. These are what `syn` and the reader recurse on, a few frames a
/// level at most. The walk keeps its own stack: a source of any depth costs it no call stack.
fn deepest(ts: proc_macro2::TokenStream) -> (usize, usize) {
    use proc_macro2::{Delimiter, Spacing, TokenTree};
    struct Level {
        tokens: std::iter::Peekable<proc_macro2::token_stream::IntoIter>,
        base: usize,
        run: usize,
        /// The run at each unclosed `<`, and at the last `|`.
        angles: Vec<usize>,
        pipe: usize,
        attr: bool,
        joint: Option<char>,
    }
    impl Level {
        fn new(ts: proc_macro2::TokenStream, base: usize) -> Level {
            Level {
                tokens: ts.into_iter().peekable(),
                base,
                run: 0,
                angles: Vec::new(),
                pipe: 0,
                attr: false,
                joint: None,
            }
        }
        fn reset(&mut self) {
            self.run = 0;
            self.angles.clear();
            self.pipe = 0;
        }
    }
    let mut deepest = (0, 0);
    let mut stack = vec![Level::new(ts, 0)];
    while let Some(level) = stack.last_mut() {
        let Some(tt) = level.tokens.next() else {
            stack.pop();
            continue;
        };
        let attr = std::mem::take(&mut level.attr);
        let joint = level.joint.take();
        let counts = match &tt {
            TokenTree::Punct(p) => {
                let c = p.as_char();
                if p.spacing() == Spacing::Joint {
                    level.joint = Some(c);
                }
                match c {
                    ';' => {
                        level.reset();
                        false
                    }
                    ',' => {
                        level.run = level.angles.last().copied().unwrap_or(0).max(level.pipe);
                        false
                    }
                    ':' | '\'' => false,
                    '#' => {
                        level.attr = true;
                        false
                    }
                    '!' if attr => {
                        level.attr = true;
                        false
                    }
                    '>' if joint == Some('=') => {
                        level.reset();
                        false
                    }
                    '>' => {
                        let closes = joint != Some('-')
                            && !(p.spacing() == Spacing::Joint
                                && matches!(level.tokens.peek(), Some(TokenTree::Punct(q)) if q.as_char() == '='));
                        if closes {
                            level.angles.pop();
                        }
                        true
                    }
                    '<' => {
                        level.run += 1;
                        level.angles.push(level.run);
                        false
                    }
                    '|' => {
                        level.run += 1;
                        level.pipe = level.run;
                        false
                    }
                    _ => true,
                }
            }
            TokenTree::Ident(i) => RUST_KEYWORDS.contains(&i.to_string().as_str()),
            TokenTree::Literal(_) => false,
            TokenTree::Group(_) => !attr,
        };
        level.run += usize::from(counts);
        let depth = level.base + level.run;
        if depth > deepest.0 {
            deepest = (depth, tt.span().start().line);
        }
        if let TokenTree::Group(g) = tt {
            let ends = g.delimiter() == Delimiter::Brace
                && match level.tokens.peek() {
                    Some(TokenTree::Ident(i)) => i != "else" && i != "as",
                    Some(TokenTree::Punct(p)) => matches!(p.as_char(), '#' | '\''),
                    Some(TokenTree::Literal(_)) => true,
                    _ => false,
                };
            if ends {
                level.reset();
            }
            stack.push(Level::new(g.stream(), depth));
        }
    }
    deepest
}

/// Rust's keywords, strict and reserved, of the 2018 edition and later: a raw identifier exists to
/// use one as a name.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];

/// Every raw identifier (`r#match`) renamed without its prefix before anything reads a name, so
/// that every position of it (declaration, type, path, pattern, field, macro argument) agrees.
///
/// `r#x` with `x` not a keyword is the identifier `x` itself, in Rust as here. One with `x` a
/// keyword is spelled `x_` (`r#type` → `type_`, `r#loop` → `loop_`), which is no keyword, so a
/// macro's arguments still parse, and which the Go spelling of functions, fields and locals then
/// starts from (`Type`, `type_`, `loop`). That spelling, and its Go spellings, must differ from
/// every other identifier of the file, in every scope (a raw `r#type_` is the name `type_` and
/// counts as one): otherwise `r#type` and `type_` would become one binding. Where one would not,
/// `x_1`, `x_2`, … is taken instead.
///
/// A type or trait the crate declares under a raw keyword is named by its visibility as a
/// function is (`pub struct r#match` → `Match`, `struct r#match` → `match_`), with `_` added while that is
/// a Rust keyword (a name inside macro arguments must parse) or another identifier of the file.
/// Spans are kept: positions and kept source stay the source's.
struct Unraw {
    names: BTreeMap<String, String>,
}

/// The names a raw identifier's spelling must differ from: every name of the file, with its Go
/// spellings.
#[derive(Default)]
struct Taken {
    names: BTreeSet<String>,
    low: BTreeSet<String>,
    up: BTreeSet<String>,
}

impl Taken {
    fn insert(&mut self, n: &str) {
        self.names.insert(n.to_string());
        self.low.insert(names::unexported(n));
        self.up.insert(names::exported(n));
    }

    fn clashes(&self, c: &str) -> bool {
        self.names.contains(c)
            || self.low.contains(&names::unexported(c))
            || self.up.contains(&names::exported(c))
    }
}

impl Unraw {
    fn of(f: &syn::File, srcs: &[String]) -> Unraw {
        let mut taken = Taken::default();
        let mut raw = BTreeSet::new();
        for src in srcs {
            if let Ok(ts) = src.parse::<proc_macro2::TokenStream>() {
                Unraw::plain_idents(ts.clone(), &mut taken);
                Unraw::raw_idents(ts, &mut raw);
            }
        }
        // `r#type_` is the plain name `type_`: it is taken as a plain one is.
        for bare in &raw {
            if !RUST_KEYWORDS.contains(&bare.as_str()) {
                taken.insert(bare);
            }
        }
        let mut out = BTreeMap::new();
        for item in &f.items {
            let (ident, vis) = match item {
                syn::Item::Struct(s) => (&s.ident, &s.vis),
                syn::Item::Enum(e) => (&e.ident, &e.vis),
                syn::Item::Trait(t) => (&t.ident, &t.vis),
                _ => continue,
            };
            // `r#Foo` is the type `Foo`, as a plain `Foo` names it too.
            if let Some(bare) = ident.to_string().strip_prefix("r#")
                && RUST_KEYWORDS.contains(&bare)
            {
                let mut go = if is_pub(vis) {
                    names::exported(bare)
                } else {
                    names::unexported(bare)
                };
                while RUST_KEYWORDS.contains(&go.as_str()) || taken.names.contains(&go) {
                    go.push('_');
                }
                taken.insert(&go);
                out.insert(bare.to_string(), go);
            }
        }
        for bare in raw {
            if out.contains_key(&bare) {
                continue;
            }
            let spelled = if RUST_KEYWORDS.contains(&bare.as_str()) {
                let mut c = format!("{bare}_");
                let mut k = 1;
                while taken.clashes(&c) {
                    c = format!("{bare}_{k}");
                    k += 1;
                }
                taken.insert(&c);
                c
            } else {
                bare.clone()
            };
            out.insert(bare, spelled);
        }
        Unraw { names: out }
    }

    /// Every name of `ts` written without `r#`, inside groups too: an identifier that is not a
    /// keyword (a keyword written plainly is the keyword, never a name).
    fn plain_idents(ts: proc_macro2::TokenStream, out: &mut Taken) {
        for tt in ts {
            match tt {
                proc_macro2::TokenTree::Ident(i) => {
                    let s = i.to_string();
                    if !s.starts_with("r#")
                        && !RUST_KEYWORDS.contains(&s.as_str())
                        && !matches!(s.as_str(), "self" | "Self" | "crate" | "super")
                    {
                        out.insert(&s);
                    }
                }
                proc_macro2::TokenTree::Group(g) => Unraw::plain_idents(g.stream(), out),
                _ => {}
            }
        }
    }

    /// The bare word of every raw identifier of `ts`.
    fn raw_idents(ts: proc_macro2::TokenStream, out: &mut BTreeSet<String>) {
        for tt in ts {
            match tt {
                proc_macro2::TokenTree::Ident(i) => {
                    if let Some(b) = i.to_string().strip_prefix("r#") {
                        out.insert(b.to_string());
                    }
                }
                proc_macro2::TokenTree::Group(g) => Unraw::raw_idents(g.stream(), out),
                _ => {}
            }
        }
    }

    fn name(&self, i: &proc_macro2::Ident) -> Option<proc_macro2::Ident> {
        let s = i.to_string();
        let bare = s.strip_prefix("r#")?;
        let n = self
            .names
            .get(bare)
            .cloned()
            .unwrap_or_else(|| format!("{bare}_"));
        Some(proc_macro2::Ident::new(&n, i.span()))
    }

    fn tokens(&self, ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        use proc_macro2::{Group, TokenTree};
        ts.into_iter()
            .map(|tt| match tt {
                TokenTree::Ident(i) => TokenTree::Ident(self.name(&i).unwrap_or(i)),
                TokenTree::Group(g) => {
                    let mut ng = Group::new(g.delimiter(), self.tokens(g.stream()));
                    ng.set_span(g.span());
                    TokenTree::Group(ng)
                }
                t => t,
            })
            .collect()
    }
}

impl VisitMut for Unraw {
    fn visit_ident_mut(&mut self, i: &mut proc_macro2::Ident) {
        if let Some(n) = self.name(i) {
            *i = n;
        }
    }

    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        syn::visit_mut::visit_macro_mut(self, m);
        m.tokens = self.tokens(std::mem::take(&mut m.tokens));
    }
}

/// What a declared type is, by its Rust name (which is its Go name).
#[derive(Debug, Clone)]
pub(crate) enum Decl {
    Struct(Vec<FieldInfo>),
    Newtype(Type),
    Enum { under: Type, variants: Vec<String> },
    Union,
}

#[derive(Debug, Clone)]
pub(crate) struct FieldInfo {
    pub rust: String,
    pub go: String,
    pub t: Type,
}

#[derive(Debug, Clone)]
pub(crate) enum ErrVariant {
    Sentinel { go: String },
    Struct(String),
    Msg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    Plain,
    Fallible,
    CommaOk,
}

#[derive(Debug, Clone)]
pub(crate) struct Sig {
    pub go: String,
    /// A function's type parameters: its own, after those of a generic struct's impl for one
    /// without a receiver. A method's come from its receiver.
    pub tparams: Vec<TypeParam>,
    pub recv: Option<Type>,
    pub params: Vec<(String, String, Type)>,
    pub results: Vec<Type>,
    pub shape: Shape,
}

impl Sig {
    /// The type of a call's value: one result, a tuple of several, or unit.
    pub fn value_type(&self) -> Type {
        match self.results.as_slice() {
            [] => Type::Unit,
            [t] => t.clone(),
            ts => Type::Tuple { elems: ts.to_vec() },
        }
    }
}

pub(crate) struct Reader {
    /// The root file's position name, for a span of no file read.
    pub file: String,
    /// The position name of each file read, by the file name its spans carry.
    pub span_files: BTreeMap<String, String>,
    /// Each file's place in the order the files were read: refusals are sorted by it, then by
    /// line.
    pub file_order: BTreeMap<String, usize>,
    pub known: BTreeSet<String>,
    pub decls: BTreeMap<String, Decl>,
    /// The closed error enum's name and its variants by Rust name.
    pub error_enum: Option<(String, BTreeMap<String, ErrVariant>)>,
    /// Message enums by name: each variant wraps the struct of its name.
    pub message_enums: BTreeSet<String>,
    /// Methods keyed by (type, Rust name); free and associated functions by (None, Rust name)
    /// and (Some(type), Rust name) with no receiver.
    pub sigs: BTreeMap<(Option<String>, String), Sig>,
    pub ptr_types: BTreeSet<String>,
    pub consts: BTreeMap<String, (String, Type)>,
    /// The literal value of each read `const`, by Go name: what proves a shift's count.
    pub const_values: BTreeMap<String, String>,
    /// Types whose `PartialEq` the crate writes by hand, by Rust name.
    pub hand_eq: BTreeSet<String>,
    /// Types whose `PartialOrd` or `Ord` the crate writes by hand, by Rust name.
    pub hand_ord: BTreeSet<String>,
    /// Types `enum Error` wraps: their `Display` is their `Error()`.
    pub error_structs: BTreeSet<String>,
    /// Unit enums whose `value()` method was absorbed.
    pub value_enums: BTreeSet<String>,
    /// Types with a `Display` rendering.
    pub displays: BTreeSet<String>,
    /// Structs and newtypes whose `#[derive(Default)]` makes Go's zero value of them.
    pub defaults: BTreeSet<String>,
    /// The crate's traits by name, read as interfaces; `None` while only the name is known.
    pub traits: BTreeMap<String, Option<TraitInfo>>,
    /// `(type, trait)` for each `impl Trait for Type` of a crate trait, `Type` a declared one.
    pub impls: BTreeSet<(String, String)>,
    /// `(type, trait)` for each `impl Trait for &Type` of a crate trait.
    pub ref_impls: BTreeSet<(String, String)>,
    /// `(type, trait)` of the impls refused for a method name another crate trait's impl for the
    /// type has too, with the reason.
    pub trait_clashes: BTreeMap<(String, String), String>,
    /// `static` items whose initialiser is refused, by Rust name, with the reason.
    pub refused_statics: BTreeMap<String, String>,
    /// Functions and methods (keyed as `sigs`) not declared because another item has their Go
    /// name, with the reason.
    pub name_clashes: BTreeMap<(Option<String>, String), String>,
    /// Whether the crate names `BTreeMap` or `BTreeSet` anywhere: a map whose Rust type the
    /// reader cannot tell may then be one that visits its keys in order.
    pub ordered_maps: bool,
    /// The fields of a `BTreeMap` (`true`) or `HashMap` (`false`) type, by struct and Go name.
    pub map_fields: BTreeMap<(String, String), bool>,
    /// Types whose `Display` impl is refused, with the reason.
    pub refused_displays: BTreeMap<String, String>,
    /// `const` items whose type or value is refused, by Rust name, with the reason.
    pub refused_consts: BTreeMap<String, String>,
    /// `static` and `static mut` items by Rust name: their Go name and type.
    pub statics: BTreeMap<String, (String, Type)>,
    /// The type parameters of each generic struct, by name.
    pub type_params: BTreeMap<String, Vec<TypeParam>>,
    /// The type parameters in scope while an item is read.
    pub scope: RefCell<Scope>,
    pub refusals: Vec<Refusal>,
    /// How the crate names its functions, types, modules and imports, for listing calls.
    pub tables: calls::Tables,
    /// The module path of each item of the file read, in order.
    pub item_modules: Vec<String>,
    /// The index of each item of the file read, by its address, to find its module.
    pub item_index: std::collections::HashMap<usize, usize>,
    /// The module path and the impl type of the item being read, for listing its calls.
    pub at: RefCell<(String, Option<String>)>,
    /// The functions refused whole, with their calls.
    pub opaque: Vec<codespec_model::Opaque>,
    /// The functions and impls refused for a name two modules declare, each with its module's
    /// path and the reason: read as opaque functions only.
    pub apart: Vec<(syn::Item, String, String)>,
    /// The tokens a read with `--spans` notes (`spans`); off otherwise.
    pub spans: spans::Shared,
}

/// The type parameters in scope while an item is read, a generic function's or impl's, and the
/// type arguments `Self` stands for in an impl of a generic struct (`impl<T> Pair<T>`: `Pair[T]`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Scope {
    pub params: Vec<TypeParam>,
    pub self_targs: Vec<Type>,
}

impl Scope {
    /// A scope of `params`, `Self` none of them.
    fn of(params: Vec<TypeParam>) -> Scope {
        Scope {
            params,
            self_targs: Vec::new(),
        }
    }
}

/// A trait of the crate read as an interface: its methods, each taking `&self`.
#[derive(Debug, Clone)]
pub(crate) struct TraitInfo {
    pub exported: bool,
    /// By Rust name: the Go name, the parameter types and the result types.
    pub methods: BTreeMap<String, (String, Vec<Type>, Vec<Type>)>,
}

/// The first attribute among `attrs` that gives serde a wire form other than the derived one,
/// as its source: `#[serde(…)]` with any key but `bound` and `crate` (`rename`, `rename_all`,
/// `skip`, `flatten`, `default`, `tag`, `untagged`, `with`, …), or an attribute of a serde helper
/// crate (`#[serde_as]`).
fn serde_wire(attrs: &[syn::Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|a| {
            let first = a
                .path()
                .segments
                .first()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            if first.starts_with("serde_") {
                return true;
            }
            if !a.path().is_ident("serde") {
                return false;
            }
            !a.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|ms| {
                ms.iter()
                    .all(|m| m.path().is_ident("bound") || m.path().is_ident("crate"))
            })
        })
        .map(src_of)
}

/// The serde attribute of a field among `fields` that sets a wire form, and where it stands.
fn serde_wire_fields(fields: &syn::Fields, of: &str) -> Option<String> {
    fields.iter().enumerate().find_map(|(k, fl)| {
        let name = fl
            .ident
            .as_ref()
            .map_or_else(|| k.to_string(), |i| i.to_string());
        serde_wire(&fl.attrs).map(|a| format!("`{a}` on field {name}{of}"))
    })
}

/// The serde attribute of the struct `s` or one of its fields that sets a wire form.
fn serde_wire_struct(s: &syn::ItemStruct) -> Option<String> {
    serde_wire(&s.attrs)
        .map(|a| format!("`{a}`"))
        .or_else(|| serde_wire_fields(&s.fields, ""))
}

/// The serde attribute of the enum `e`, a variant or a variant's field that sets a wire form.
fn serde_wire_enum(e: &syn::ItemEnum) -> Option<String> {
    serde_wire(&e.attrs).map(|a| format!("`{a}`")).or_else(|| {
        e.variants.iter().find_map(|v| {
            serde_wire(&v.attrs)
                .map(|a| format!("`{a}` on variant {}", v.ident))
                .or_else(|| serde_wire_fields(&v.fields, &format!(" of variant {}", v.ident)))
        })
    })
}

fn is_pub(v: &syn::Visibility) -> bool {
    matches!(v, syn::Visibility::Public(_))
}

fn doc(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for a in attrs {
        if !a.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &a.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
        {
            let v = s.value();
            lines.push(v.strip_prefix(' ').unwrap_or(&v).to_string());
        }
    }
    lines.join("\n").trim().to_string()
}

/// The std items the reader's type and call tables name, each by every full path that names it,
/// its canonical path first. A bare name stands for a prelude item only where no `use` and no
/// item of the crate takes it. A path is matched whole, after its module's `use` declarations
/// resolve it: `tokio::sync::broadcast::Sender` is not `std::sync::mpsc::Sender`.
const STD_ITEMS: &[&[&str]] = &[
    &["std::string::String", "String", "alloc::string::String"],
    &["std::vec::Vec", "Vec", "alloc::vec::Vec"],
    &["std::boxed::Box", "Box", "alloc::boxed::Box"],
    &["std::option::Option", "Option", "core::option::Option"],
    &["std::result::Result", "Result", "core::result::Result"],
    &["std::default::Default", "Default", "core::default::Default"],
    &[
        "std::collections::HashMap",
        "std::collections::hash_map::HashMap",
    ],
    &[
        "std::collections::BTreeMap",
        "std::collections::btree_map::BTreeMap",
        "alloc::collections::BTreeMap",
        "alloc::collections::btree_map::BTreeMap",
    ],
    &["std::rc::Rc", "alloc::rc::Rc"],
    &["std::cell::RefCell", "core::cell::RefCell"],
    &["std::sync::mpsc::Sender"],
    &["std::sync::mpsc::SyncSender"],
    &["std::sync::mpsc::Receiver"],
    &["std::any::Any", "core::any::Any"],
    &["std::fmt::Display", "core::fmt::Display"],
    &["std::error::Error", "core::error::Error"],
];

/// The last segment of a path type, with its generic arguments.
fn last_seg(t: &syn::Type) -> Option<(&syn::Ident, Vec<&syn::Type>)> {
    let syn::Type::Path(p) = t else { return None };
    if p.qself.is_some() {
        return None;
    }
    let seg = p.path.segments.last()?;
    let args = match &seg.arguments {
        syn::PathArguments::None => Vec::new(),
        syn::PathArguments::AngleBracketed(a) => {
            let mut out = Vec::new();
            for g in &a.args {
                match g {
                    syn::GenericArgument::Type(t) => out.push(t),
                    _ => return None,
                }
            }
            out
        }
        syn::PathArguments::Parenthesized(_) => return None,
    };
    Some((&seg.ident, args))
}

/// The ident of a path type without generic arguments.
fn plain_ident(t: &syn::Type) -> Option<String> {
    match last_seg(t) {
        Some((i, a)) if a.is_empty() => Some(i.to_string()),
        _ => None,
    }
}

/// The name of a primitive type (`str`, `i64`): a path type of one segment, no arguments.
fn prim_ident(t: &syn::Type) -> Option<String> {
    match t {
        syn::Type::Path(p) if p.qself.is_none() => p.path.get_ident().map(|i| i.to_string()),
        _ => None,
    }
}

/// `"s"` quoted as Go's `strconv.Quote` writes it for the characters a source holds.
pub(crate) fn go_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\v"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn int(width: IntWidth, signed: bool) -> Type {
    Type::Int { width, signed }
}

pub(crate) fn int_type(name: &str) -> Option<Type> {
    Some(match name {
        "i8" => int(IntWidth::W8, true),
        "i16" => int(IntWidth::W16, true),
        "i32" => int(IntWidth::W32, true),
        "i64" => int(IntWidth::W64, true),
        "isize" => int(IntWidth::Platform, true),
        "u8" => int(IntWidth::W8, false),
        "u16" => int(IntWidth::W16, false),
        "u32" => int(IntWidth::W32, false),
        "u64" => int(IntWidth::W64, false),
        "usize" => int(IntWidth::Platform, false),
        _ => return None,
    })
}

/// A unit enum's `value(&self)` method mapping every variant to a literal: the underlying type and
/// the values in variant order.
fn value_idiom(
    f: &syn::ImplItemFn,
    enum_name: &str,
    variants: &[String],
) -> Option<(Type, Vec<Expr>)> {
    if f.sig.ident != "value" || f.sig.inputs.len() != 1 {
        return None;
    }
    let syn::ReturnType::Type(_, rt) = &f.sig.output else {
        return None;
    };
    let under = match &**rt {
        syn::Type::Reference(r) if prim_ident(&r.elem).as_deref() == Some("str") => Type::String,
        t => int_type(&prim_ident(t)?)?,
    };
    let [syn::Stmt::Expr(syn::Expr::Match(m), None)] = f.block.stmts.as_slice() else {
        return None;
    };
    let mut values: Vec<Option<Expr>> = vec![None; variants.len()];
    for arm in &m.arms {
        let syn::Pat::Path(p) = &arm.pat else {
            return None;
        };
        let segs: Vec<String> = p
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let [t, v] = segs.as_slice() else { return None };
        if t != enum_name && t != "Self" {
            return None;
        }
        let i = variants.iter().position(|x| x == v)?;
        let syn::Expr::Lit(l) = &*arm.body else {
            return None;
        };
        let value = match (&l.lit, &under) {
            (syn::Lit::Str(s), Type::String) => go_quote(&s.value()),
            (syn::Lit::Int(n), Type::Int { .. }) => n.base10_digits().to_string(),
            _ => return None,
        };
        values[i] = Some(Expr::Lit {
            value,
            t: under.clone(),
        });
    }
    let values: Option<Vec<Expr>> = values.into_iter().collect();
    Some((under, values?))
}

impl Reader {
    fn new(file: &str) -> Reader {
        Reader {
            file: file.to_string(),
            span_files: BTreeMap::new(),
            file_order: BTreeMap::new(),
            known: BTreeSet::new(),
            decls: BTreeMap::new(),
            error_enum: None,
            message_enums: BTreeSet::new(),
            sigs: BTreeMap::new(),
            ptr_types: BTreeSet::new(),
            consts: BTreeMap::new(),
            const_values: BTreeMap::new(),
            hand_eq: BTreeSet::new(),
            hand_ord: BTreeSet::new(),
            error_structs: BTreeSet::new(),
            value_enums: BTreeSet::new(),
            displays: BTreeSet::new(),
            defaults: BTreeSet::new(),
            traits: BTreeMap::new(),
            impls: BTreeSet::new(),
            ref_impls: BTreeSet::new(),
            trait_clashes: BTreeMap::new(),
            refused_statics: BTreeMap::new(),
            refused_consts: BTreeMap::new(),
            refused_displays: BTreeMap::new(),
            name_clashes: BTreeMap::new(),
            ordered_maps: false,
            map_fields: BTreeMap::new(),
            statics: BTreeMap::new(),
            type_params: BTreeMap::new(),
            scope: RefCell::new(Scope::default()),
            refusals: Vec::new(),
            tables: calls::Tables::default(),
            item_modules: Vec::new(),
            item_index: std::collections::HashMap::new(),
            at: RefCell::new(("crate".into(), None)),
            opaque: Vec::new(),
            apart: Vec::new(),
            spans: spans::Shared::default(),
        }
    }

    /// Note for `--spans` the body node `x` and the identifier spelling its name.
    pub(crate) fn note(&self, x: &Expr, name: proc_macro2::Span) {
        self.spans.borrow_mut().node(x, name);
    }

    /// Note for `--spans` the struct literal `x` and the identifier of each field it kept, with
    /// whether it is written in shorthand.
    pub(crate) fn note_literal(&self, x: &Expr, fields: &[(proc_macro2::Span, bool)]) {
        self.spans.borrow_mut().literal(x, fields);
    }

    /// Whether a read notes spans.
    pub(crate) fn noting(&self) -> bool {
        self.spans.borrow().is_on()
    }

    /// Note for `--spans` each leaf of a `use` item standing in module `module` that names an
    /// item of the crate: its path is the crate's (`crate::`, `self::`, `super::` or a module of
    /// the crate) and leads to a module, not into a type.
    fn note_use(&self, module: &str, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.note_use(module, &p.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(syn::UseName { ident })
            | syn::UseTree::Rename(syn::UseRename { ident, .. }) => {
                let leaf = ident.to_string();
                if leaf == "self" {
                    return;
                }
                let mut path = prefix.clone();
                path.push(leaf.clone());
                let full = self.tables.resolve(module, None, &path);
                if full.len() > 1
                    && full[0] == "crate"
                    && self
                        .tables
                        .modules
                        .contains(&full[..full.len() - 1].join("::"))
                {
                    self.spans.borrow_mut().use_leaf(leaf, ident.span());
                }
            }
            syn::UseTree::Group(g) => {
                for t in &g.items {
                    self.note_use(module, t, prefix);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }

    /// `f` run with `scope` the type parameters in scope.
    pub fn in_scope<T>(&self, scope: Scope, f: impl FnOnce() -> T) -> T {
        let outer = self.scope.replace(scope);
        let out = f();
        self.scope.replace(outer);
        out
    }

    /// Make `item`, an item of the file read, the one being read: the paths it writes resolve
    /// through its module's `use` declarations.
    pub fn enter(&self, item: &syn::Item) {
        let module = self
            .item_index
            .get(&(item as *const syn::Item as usize))
            .and_then(|k| self.item_modules.get(*k))
            .cloned()
            .unwrap_or_else(|| "crate".into());
        let self_ty = match item {
            syn::Item::Impl(im) => impl_type_name(&im.self_ty),
            _ => None,
        };
        *self.at.borrow_mut() = (module, self_ty);
    }

    /// The full path the segments `segs` name in the item being read: through its module's
    /// `use` declarations, `crate` first for the crate's own items, as written for a name
    /// neither declares (the prelude's).
    pub fn resolve(&self, segs: &[String]) -> Vec<String> {
        let (module, self_ty) = self.at.borrow().clone();
        self.tables.resolve(&module, self_ty.as_deref(), segs)
    }

    /// The full path `p` names, its generic arguments left out.
    pub fn full_path(&self, p: &syn::Path) -> Vec<String> {
        let segs: Vec<String> = p.segments.iter().map(|s| s.ident.to_string()).collect();
        self.resolve(&segs)
    }

    /// The std item of the reader's tables the full path `full` names: its first path in
    /// [`STD_ITEMS`].
    pub fn std_item(&self, full: &[String]) -> Option<&'static str> {
        let joined = full.join("::");
        STD_ITEMS
            .iter()
            .find(|ps| ps.contains(&joined.as_str()))
            .map(|ps| ps[0])
    }

    /// The std item of the reader's tables the path `p` names.
    pub fn std_path(&self, p: &syn::Path) -> Option<&'static str> {
        self.std_item(&self.full_path(p))
    }

    /// The std item of the reader's tables and the function's name a call path `T::f` names, by
    /// the full path of `T`: `HashMap::new` with `use std::collections::HashMap;` is
    /// `("std::collections::HashMap", "new")`.
    pub fn std_call(&self, p: &syn::Path) -> Option<(&'static str, String)> {
        if p.segments.len() < 2 {
            return None;
        }
        let full = self.full_path(p);
        let (f, ty) = full.split_last()?;
        Some((self.std_item(ty)?, f.clone()))
    }

    /// The std item of the reader's tables a path type names, with its type arguments.
    pub fn std_type<'t>(&self, t: &'t syn::Type) -> Option<(&'static str, Vec<&'t syn::Type>)> {
        let syn::Type::Path(p) = t else { return None };
        let (_, args) = last_seg(t)?;
        Some((self.std_path(&p.path)?, args))
    }

    /// The name of the crate's own item the full path `full` names: `crate`, the crate's
    /// modules, then the name.
    pub fn own_item(&self, full: &[String]) -> Option<String> {
        let (last, prefix) = full.split_last()?;
        (prefix.first().map(String::as_str) == Some("crate")
            && (prefix.len() == 1 || self.tables.modules.contains(&prefix.join("::"))))
        .then(|| last.clone())
    }

    /// The crate's own type a path type without type arguments names.
    pub fn own_type(&self, t: &syn::Type) -> Option<String> {
        let syn::Type::Path(p) = t else { return None };
        if p.qself.is_some() || !last_seg(t)?.1.is_empty() {
            return None;
        }
        self.own_item(&self.full_path(&p.path))
    }

    /// The full path a name of one segment, written in the item being read, takes from a `use`
    /// of something outside the crate: such a name is not the crate's item of that name.
    pub fn imported_from_outside(&self, name: &str) -> Option<String> {
        let full = self.resolve(&[name.to_string()]);
        (full.first().map(String::as_str) != Some("crate") && full != [name])
            .then(|| full.join("::"))
    }

    /// Whether `#[derive(…)]` among `attrs` names the std trait `std` by its full path.
    fn derives(&self, attrs: &[syn::Attribute], std: &str) -> bool {
        attrs.iter().any(|a| {
            a.path().is_ident("derive")
                && a.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|ps| ps.iter().any(|p| self.std_path(p) == Some(std)))
        })
    }

    /// The constraint of the type parameter `name` in scope.
    pub fn constraint_of(&self, name: &str) -> Option<Type> {
        self.scope
            .borrow()
            .params
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.constraint.clone())
    }

    fn param_in_scope(&self, name: &str) -> bool {
        self.scope.borrow().params.iter().any(|p| p.name == name)
    }

    /// The names of the type parameters `g` declares: a lifetime or const parameter, or a default,
    /// is refused.
    fn tparam_names(&self, g: &syn::Generics) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for p in &g.params {
            match p {
                syn::GenericParam::Lifetime(l) => {
                    return Err(format!(
                        "a lifetime parameter ({}) is outside cir/1: a Go value has no lifetime",
                        l.lifetime
                    ));
                }
                syn::GenericParam::Const(c) => {
                    return Err(format!(
                        "a const parameter ({}) is outside cir/1: Go's type parameters are types",
                        c.ident
                    ));
                }
                syn::GenericParam::Type(t) => {
                    if t.default.is_some() {
                        return Err(format!(
                            "the type parameter {} has a default, which Go's has no form for",
                            t.ident
                        ));
                    }
                    names.push(t.ident.to_string());
                }
            }
        }
        Ok(names)
    }

    /// The constraint one bound list makes: none is `any`, one trait of the crate read as an
    /// interface is that interface. Anything else (a std trait such as `Clone`, `Copy`,
    /// `PartialEq` or `Ord`, two bounds, a lifetime, `?Sized`, an associated type binding) has no
    /// `cir/1` constraint: `cir/1` constrains a type parameter by `any` or one interface.
    fn bound(
        &self,
        param: &str,
        bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
    ) -> Result<Type, String> {
        let mut it = bounds.iter();
        let (b, None) = (it.next(), it.next()) else {
            return Err(format!(
                "the bound `{}` on {param}: cir/1 constrains a type parameter by `any` or one interface, not by several bounds",
                src_of(bounds)
            ));
        };
        let Some(b) = b else { return Ok(Type::Any) };
        let no_form = || {
            format!(
                "the bound `{}` on {param}, which cir/1 has no constraint for: it constrains a type parameter by `any` or one of the crate's traits read as an interface",
                src_of(b)
            )
        };
        let syn::TypeParamBound::Trait(tb) = b else {
            return Err(no_form());
        };
        if tb.lifetimes.is_some() || tb.maybe.is_some() || tb.path.segments.len() != 1 {
            return Err(no_form());
        }
        let seg = &tb.path.segments[0];
        match &seg.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(a)
                if a.args.iter().any(|g| {
                    matches!(
                        g,
                        syn::GenericArgument::AssocType(_) | syn::GenericArgument::AssocConst(_)
                    )
                }) =>
            {
                return Err(format!(
                    "the bound `{}` on {param} binds an associated type, which cir/1 has no form for",
                    src_of(b)
                ));
            }
            _ => return Err(no_form()),
        }
        let Some(name) = self.own_item(&self.full_path(&tb.path)) else {
            return Err(no_form());
        };
        match self.traits.get(&name) {
            Some(Some(_)) => Ok(Type::Named {
                name,
                targs: Vec::new(),
            }),
            _ => Err(no_form()),
        }
    }

    /// The type parameters of `g`, named `names`, each with its constraint: from its inline bound
    /// or one `where T: Trait` of a parameter without an inline bound.
    fn constraints(&self, g: &syn::Generics, names: &[String]) -> Result<Vec<TypeParam>, String> {
        let mut out = Vec::new();
        let mut bounded = BTreeSet::new();
        for p in &g.params {
            if let syn::GenericParam::Type(t) = p {
                let name = t.ident.to_string();
                if !t.bounds.is_empty() {
                    bounded.insert(name.clone());
                }
                out.push(TypeParam {
                    constraint: self.bound(&name, &t.bounds)?,
                    name,
                });
            }
        }
        if let Some(w) = &g.where_clause {
            for pred in &w.predicates {
                let simple = match pred {
                    syn::WherePredicate::Type(pt) if pt.lifetimes.is_none() => match &pt.bounded_ty
                    {
                        syn::Type::Path(p) if p.qself.is_none() => p
                            .path
                            .get_ident()
                            .map(|i| i.to_string())
                            .filter(|n| names.contains(n) && !bounded.contains(n))
                            .map(|n| (n, &pt.bounds)),
                        _ => None,
                    },
                    _ => None,
                };
                let Some((name, bounds)) = simple else {
                    return Err(format!(
                        "the `where` clause `{}`: the reader reads only `T: Trait` of a type parameter with no other bound",
                        src_of(pred)
                    ));
                };
                bounded.insert(name.clone());
                let c = self.bound(&name, bounds)?;
                if let Some(tp) = out.iter_mut().find(|tp| tp.name == name) {
                    tp.constraint = c;
                }
            }
        }
        Ok(out)
    }

    /// The type of an impl and the scope its items are read in: a generic struct's impl names
    /// each of the struct's type parameters once, in order, with the struct's constraints, as a
    /// Go method's receiver `Pair[T]` binds them.
    fn impl_scope(&self, im: &syn::ItemImpl) -> Result<(String, Scope), String> {
        let not_declared = || "an impl for a type other than a declared one".to_string();
        let (id, args) = last_seg(&im.self_ty).ok_or_else(not_declared)?;
        let name = id.to_string();
        let syn::Type::Path(sp) = &*im.self_ty else {
            return Err(not_declared());
        };
        if self.own_item(&self.full_path(&sp.path)).as_deref() != Some(name.as_str()) {
            return Err(format!(
                "an impl for a type other than a declared one: `{}` is not a type of the crate",
                src_of(&im.self_ty)
            ));
        }
        let Some(tps) = self.type_params.get(&name) else {
            let declared = self.decls.contains_key(&name)
                || self.error_enum.as_ref().is_some_and(|(e, _)| *e == name)
                || self.message_enums.contains(&name);
            // `impl Tr for i64`, `impl Tr for Vec<T>`: a type the module does not declare has no
            // methods in Go (`impl Tr for &T` has no path, so `last_seg` refused it above).
            if !declared {
                return Err(not_declared());
            }
            if !args.is_empty() || !im.generics.params.is_empty() {
                return Err(format!(
                    "an impl with type parameters of {name}, which has none"
                ));
            }
            return Ok((name, Scope::default()));
        };
        if im.trait_.is_some() {
            return Err(format!(
                "an impl of a trait for the generic struct {name}: the reader reads inherent impls of a generic struct only"
            ));
        }
        let names = self.tparam_names(&im.generics)?;
        let given: Vec<Option<String>> = args.iter().map(|a| plain_ident(a)).collect();
        if given.len() != tps.len()
            || names.len() != tps.len()
            || given.iter().zip(&names).any(|(g, n)| g.as_ref() != Some(n))
        {
            return Err(format!(
                "impl {}: a Go method of {name} binds each of its type parameters once, in order",
                src_of(&im.self_ty)
            ));
        }
        let params = self.constraints(&im.generics, &names)?;
        if params
            .iter()
            .zip(tps)
            .any(|(p, d)| p.constraint != d.constraint)
        {
            return Err(format!(
                "impl {}: bounds other than {name}'s own, which a Go method has no form for",
                src_of(&im.self_ty)
            ));
        }
        let self_targs = names
            .iter()
            .map(|n| Type::Param { name: n.clone() })
            .collect();
        Ok((name, Scope { params, self_targs }))
    }

    /// The first named type `t` mentions that the module does not declare: a struct, newtype,
    /// enum or union refused, or a crate trait not read as an interface.
    fn undeclared(&self, t: &Type) -> Option<String> {
        match t {
            Type::Named { name, targs } => {
                let declared = self.decls.contains_key(name)
                    || matches!(self.traits.get(name), Some(Some(_)))
                    || self.message_enums.contains(name);
                if !declared {
                    return Some(name.clone());
                }
                targs.iter().find_map(|a| self.undeclared(a))
            }
            Type::Ptr { elem } | Type::Slice { elem } | Type::Array { elem, .. } => {
                self.undeclared(elem)
            }
            Type::Chan { elem, .. } => self.undeclared(elem),
            Type::Map { key, elem } => self.undeclared(key).or_else(|| self.undeclared(elem)),
            Type::Tuple { elems } => elems.iter().find_map(|e| self.undeclared(e)),
            Type::Func { params, results } => params
                .iter()
                .chain(results)
                .find_map(|e| self.undeclared(e)),
            _ => None,
        }
    }

    /// Whether `t` is a crate trait read as an interface.
    pub fn is_iface(&self, t: &Type) -> Option<&TraitInfo> {
        match t {
            Type::Named { name, targs } if targs.is_empty() => {
                self.traits.get(name).and_then(Option::as_ref)
            }
            _ => None,
        }
    }

    /// The crate trait a `dyn` type names: one bound, a path to a trait of the crate.
    fn dyn_trait(&self, t: &syn::TypeTraitObject) -> Option<String> {
        let mut bounds = t.bounds.iter();
        let (Some(syn::TypeParamBound::Trait(b)), None) = (bounds.next(), bounds.next()) else {
            return None;
        };
        if b.lifetimes.is_some() || b.maybe.is_some() {
            return None;
        }
        if b.path
            .segments
            .iter()
            .any(|s| !matches!(s.arguments, syn::PathArguments::None))
        {
            return None;
        }
        let n = self.own_item(&self.full_path(&b.path))?;
        self.traits.contains_key(&n).then_some(n)
    }

    /// Whether a `dyn` type is `std::any::Any`.
    fn dyn_any(&self, t: &syn::Type) -> bool {
        let syn::Type::TraitObject(o) = t else {
            return false;
        };
        let mut bounds = o.bounds.iter();
        matches!(
            (bounds.next(), bounds.next()),
            (Some(syn::TypeParamBound::Trait(b)), None)
                if b.lifetimes.is_none()
                    && b.path.segments.iter().all(|s| s.arguments.is_none())
                    && self.std_path(&b.path) == Some("std::any::Any")
        )
    }

    pub fn pos(&self, span: proc_macro2::Span) -> String {
        let file = self.span_files.get(&span.file()).unwrap_or(&self.file);
        format!("{file}:{}", span.start().line)
    }

    /// Where a position `<file>:<line>` sorts: by the order the files were read, then by line.
    fn pos_key(&self, p: &str) -> (usize, usize) {
        let (file, line) = p.rsplit_once(':').unwrap_or((p, ""));
        (
            self.file_order.get(file).copied().unwrap_or(0),
            line.parse().unwrap_or(0),
        )
    }

    fn refuse(&mut self, span: proc_macro2::Span, what: &str, reason: impl Into<String>) {
        let pos = self.pos(span);
        self.refusals.push(Refusal {
            pos,
            what: what.to_string(),
            reason: reason.into(),
        });
    }

    /// A Rust type in a field, parameter or local. `self_ty` is the impl's type, for `Self`.
    pub fn ty(&self, t: &syn::Type, self_ty: Option<&str>) -> Result<Type, String> {
        match t {
            syn::Type::Reference(r) => {
                if r.mutability.is_some() {
                    return Err("a mutable reference `&mut T` is outside cir/1".into());
                }
                if prim_ident(&r.elem).as_deref() == Some("str") {
                    return Ok(Type::String);
                }
                self.ty(&r.elem, self_ty)
            }
            syn::Type::Paren(p) => self.ty(&p.elem, self_ty),
            syn::Type::Tuple(tt) if tt.elems.is_empty() => Ok(Type::Unit),
            // `dyn Trait` of a crate trait, behind `&` or `Box`: Go's interface value. Its
            // methods take `&self` only, so neither the borrow nor the box is observable.
            syn::Type::TraitObject(o) => match self.dyn_trait(o) {
                Some(name) => Ok(Type::Named {
                    targs: Vec::new(),
                    name,
                }),
                None => Err(
                    "a trait object (`dyn`) of other than a trait of the crate is outside cir/1"
                        .into(),
                ),
            },
            syn::Type::Array(a) => {
                let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) = &a.len
                else {
                    return Err("an array whose length is not an integer literal".into());
                };
                let len: u64 = n.base10_parse().map_err(|e| e.to_string())?;
                Ok(Type::Array {
                    len,
                    elem: Box::new(self.ty(&a.elem, self_ty)?),
                })
            }
            // `fn(A) -> R`: Go's `func(A) R`, a function value of the crate's plain functions.
            syn::Type::FnPtr(f) => {
                if f.lifetimes.is_some()
                    || f.unsafety.is_some()
                    || f.abi.is_some()
                    || f.variadic.is_some()
                {
                    return Err("a function pointer type other than a plain `fn(…) -> …`".into());
                }
                let params = f
                    .inputs
                    .iter()
                    .map(|a| self.ty(&a.ty, self_ty))
                    .collect::<Result<Vec<_>, _>>()?;
                let results = match &f.output {
                    syn::ReturnType::Default => Vec::new(),
                    syn::ReturnType::Type(_, t) => match &**t {
                        syn::Type::Tuple(tt) => tt
                            .elems
                            .iter()
                            .map(|e| self.ty(e, self_ty))
                            .collect::<Result<Vec<_>, _>>()?,
                        t => vec![self.ty(t, self_ty)?],
                    },
                };
                if results.iter().any(|t| matches!(t, Type::Unit)) {
                    return Err("a function pointer type giving `()` inside a tuple".into());
                }
                Ok(Type::Func { params, results })
            }
            syn::Type::ImplTrait(_) => Err("an `impl Trait` type is outside cir/1".into()),
            syn::Type::Path(p) => {
                // A type parameter in scope; `T::Item` or `<T as Trait>::Item`, an associated
                // type, has no `cir/1` form.
                let first = p.path.segments.first().map(|s| s.ident.to_string());
                if p.qself.is_some()
                    || (p.path.segments.len() > 1
                        && first.as_deref().is_some_and(|f| self.param_in_scope(f)))
                {
                    return Err(format!(
                        "the associated type `{}` is outside cir/1",
                        src_of(t)
                    ));
                }
                if p.path.segments.len() == 1
                    && p.path.segments[0].arguments.is_none()
                    && let Some(f) = first.filter(|f| self.param_in_scope(f))
                {
                    return Ok(Type::Param { name: f });
                }
                let Some((id, args)) = last_seg(t) else {
                    return Err(format!("type `{}` is outside cir/1", src_of(t)));
                };
                let name = id.to_string();
                let one = |args: &[&syn::Type]| -> Result<Type, String> {
                    match args {
                        [a] => self.ty(a, self_ty),
                        _ => Err(format!("`{name}` without exactly one type argument")),
                    }
                };
                if p.path.is_ident("Self") {
                    return match self_ty {
                        Some(s) => Ok(Type::Named {
                            targs: self.scope.borrow().self_targs.clone(),
                            name: s.into(),
                        }),
                        None => Err("`Self` outside an impl".into()),
                    };
                }
                // A path is mapped by the full path it names, never by its last segment.
                let full = self.full_path(&p.path);
                // A primitive type: one segment no `use` and no item of the crate takes.
                if full == [name.clone()] && args.is_empty() {
                    if let Some(it) = int_type(&name) {
                        return Ok(it);
                    }
                    match name.as_str() {
                        "bool" => return Ok(Type::Bool),
                        "str" => return Ok(Type::String),
                        "f64" => {
                            return Ok(Type::Float {
                                bits: FloatBits::W64,
                            });
                        }
                        "f32" => {
                            return Ok(Type::Float {
                                bits: FloatBits::W32,
                            });
                        }
                        _ => {}
                    }
                }
                if let Some(std) = self.std_item(&full) {
                    return match (std, args.as_slice()) {
                        ("std::string::String", []) => Ok(Type::String),
                        // `Rc<dyn Any>`: Go's `any`, a shared value that is never mutated.
                        ("std::rc::Rc", [inner]) if self.dyn_any(inner) => Ok(Type::Any),
                        // `Rc<(SyncSender<T>, Receiver<T>)>`, the Rust writer's channel with both
                        // ends: Go's `chan T`.
                        ("std::rc::Rc", [syn::Type::Tuple(tt)]) => {
                            let ends: Vec<_> = tt.elems.iter().map(|e| self.std_type(e)).collect();
                            match ends.as_slice() {
                                [
                                    Some(("std::sync::mpsc::SyncSender", sa)),
                                    Some(("std::sync::mpsc::Receiver", ra)),
                                ] if sa.len() == 1 && ra.len() == 1 => {
                                    let elem = self.ty(sa[0], self_ty)?;
                                    if self.ty(ra[0], self_ty)? != elem {
                                        return Err(
                                            "a channel whose two ends carry different types".into()
                                        );
                                    }
                                    Ok(Type::Chan {
                                        elem: Box::new(elem),
                                        dir: codespec_model::ChanDir::Both,
                                    })
                                }
                                _ => Err(
                                    "`Rc` of a tuple other than `(SyncSender<T>, Receiver<T>)` of `std::sync::mpsc`".into()
                                ),
                            }
                        }
                        ("std::boxed::Box", [syn::Type::TraitObject(o)]) => match self.dyn_trait(o) {
                            Some(name) => Ok(Type::Named {
                                targs: Vec::new(),
                                name,
                            }),
                            None => Err("a trait object (`dyn`) of other than a trait of the crate is outside cir/1".into()),
                        },
                        ("std::vec::Vec", a) => Ok(Type::Slice {
                            elem: Box::new(one(a)?),
                        }),
                        ("std::collections::HashMap" | "std::collections::BTreeMap", [k, v]) => {
                            Ok(Type::Map {
                                key: Box::new(self.ty(k, self_ty)?),
                                elem: Box::new(self.ty(v, self_ty)?),
                            })
                        }
                        ("std::rc::Rc", [inner]) => match self.std_type(inner) {
                            Some(("std::cell::RefCell", ia)) if ia.len() == 1 => {
                                let elem = self.ty(ia[0], self_ty)?;
                                match &elem {
                                    Type::Named { name, .. }
                                        if matches!(self.decls.get(name), Some(Decl::Struct(_))) =>
                                    {
                                        Ok(Type::Ptr {
                                            elem: Box::new(elem),
                                        })
                                    }
                                    _ => Err(
                                        "`Rc<RefCell<T>>` of a type that is not a struct".into()
                                    ),
                                }
                            }
                            _ => Err(
                                "`Rc` other than `Rc<RefCell<T>>` is outside cir/1".into()
                            ),
                        },
                        ("std::sync::mpsc::Sender", [m]) => {
                            let elem = match self.own_type(m) {
                                Some(n) if self.message_enums.contains(&n) => Type::Any,
                                _ => self.ty(m, self_ty)?,
                            };
                            Ok(Type::Chan {
                                elem: Box::new(elem),
                                dir: codespec_model::ChanDir::Send,
                            })
                        }
                        ("std::option::Option" | "std::result::Result", _) => Err(format!(
                            "`{name}` other than as a function's result is outside cir/1"
                        )),
                        _ => Err(format!("type `{}` is outside cir/1", src_of(t))),
                    };
                }
                let Some(n) = self.own_item(&full) else {
                    return Err(format!(
                        "type `{}` (`{}`) is outside cir/1: the reader maps a path by the full path it names",
                        src_of(t),
                        full.join("::")
                    ));
                };
                let n = n.as_str();
                let a = args.as_slice();
                if a.is_empty() && self.error_enum.as_ref().is_some_and(|(e, _)| e == n) {
                    return Ok(Type::Error);
                }
                // A generic struct, with as many type arguments as it declares parameters.
                if self.type_params.contains_key(n) && self.decls.contains_key(n) {
                    let declared = self.type_params[n].len();
                    if a.len() != declared {
                        return Err(format!(
                            "the generic struct {n} with {} type arguments, where it declares {declared}",
                            a.len()
                        ));
                    }
                    return Ok(Type::Named {
                        targs: a
                            .iter()
                            .map(|x| self.ty(x, self_ty))
                            .collect::<Result<Vec<_>, _>>()?,
                        name: n.into(),
                    });
                }
                if a.is_empty() && self.decls.contains_key(n) {
                    return Ok(Type::Named {
                        targs: Vec::new(),
                        name: n.into(),
                    });
                }
                if a.is_empty() && self.known.contains(n) {
                    return Err(format!("type {n}, which the reader could not map"));
                }
                Err(format!("type `{}` is outside cir/1", src_of(t)))
            }
            _ => Err(format!("type `{}` is outside cir/1", src_of(t))),
        }
    }

    /// A function's results and its shape.
    fn results(
        &self,
        out: &syn::ReturnType,
        self_ty: Option<&str>,
    ) -> Result<(Vec<Type>, Shape), String> {
        let syn::ReturnType::Type(_, t) = out else {
            return Ok((Vec::new(), Shape::Plain));
        };
        let tuple = |t: &syn::Type| -> Result<Vec<Type>, String> {
            match t {
                syn::Type::Tuple(tt) => tt.elems.iter().map(|e| self.ty(e, self_ty)).collect(),
                t => Ok(vec![self.ty(t, self_ty)?]),
            }
        };
        if let Some((id, args)) = self.std_type(t) {
            if id == "std::result::Result" {
                let [ok, err] = args.as_slice() else {
                    return Err("`Result` without two type arguments".into());
                };
                let is_error = match (&self.error_enum, self.own_type(err)) {
                    (Some((e, _)), Some(n)) => *e == n,
                    _ => false,
                };
                if !is_error {
                    return Err(format!(
                        "`Result<_, {}>`: only the crate's closed `enum Error` maps to Go's error",
                        src_of(err)
                    ));
                }
                let mut rs = tuple(ok)?;
                rs.push(Type::Error);
                return Ok((rs, Shape::Fallible));
            }
            if id == "std::option::Option" {
                let [inner] = args.as_slice() else {
                    return Err("`Option` without one type argument".into());
                };
                return Ok((vec![self.ty(inner, self_ty)?, Type::Bool], Shape::CommaOk));
            }
        }
        Ok((tuple(t)?, Shape::Plain))
    }

    /// Whether the derived `Default` of a value of `t` is Go's zero value of it.
    fn zero_default(&self, t: &Type) -> bool {
        match t {
            Type::Int { .. }
            | Type::Float { .. }
            | Type::Bool
            | Type::String
            | Type::Slice { .. }
            | Type::Map { .. } => true,
            Type::Array { elem, len } => *len <= 32 && self.zero_default(elem),
            Type::Named { name, targs } => targs.is_empty() && self.defaults.contains(name),
            _ => false,
        }
    }

    /// A trait of the crate as an interface: no generics, supertraits or items other than
    /// methods without a default body that take `&self` and give plain results.
    fn trait_info(&self, t: &syn::ItemTrait) -> Result<TraitInfo, String> {
        if !t.generics.params.is_empty() || t.generics.where_clause.is_some() {
            return Err("a generic trait is outside cir/1".into());
        }
        if !t.supertraits.is_empty() {
            return Err("a trait with supertraits is outside cir/1".into());
        }
        if t.unsafety.is_some() || t.modifiers.auto_token.is_some() {
            return Err("an unsafe or auto trait is outside cir/1".into());
        }
        let exported = is_pub(&t.vis);
        let mut methods = BTreeMap::new();
        for item in &t.items {
            let syn::TraitItem::Fn(f) = item else {
                return Err("a trait item other than a method is outside cir/1".into());
            };
            if f.default.is_some() {
                return Err(format!(
                    "the trait method {} has a default body, which Go's interface has no form for",
                    f.sig.ident
                ));
            }
            let mut inputs = f.sig.inputs.iter();
            match inputs.next() {
                Some(syn::FnArg::Receiver(r))
                    if matches!(r.kind, syn::ReceiverKind::Reference(_, _, None)) => {}
                _ => {
                    return Err(format!(
                        "the trait method {} takes other than `&self`",
                        f.sig.ident
                    ));
                }
            }
            if !f.sig.generics.params.is_empty()
                || f.sig.asyncness.is_some()
                || matches!(f.sig.safety, syn::Safety::Unsafe(_))
            {
                return Err(format!(
                    "the trait method {} is generic, async or unsafe",
                    f.sig.ident
                ));
            }
            let params = inputs
                .map(|a| match a {
                    syn::FnArg::Typed(pt) => self.ty(&pt.ty, None),
                    syn::FnArg::Receiver(_) => Err("a second receiver".into()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            let (results, shape) = self.results(&f.sig.output, None)?;
            if shape != Shape::Plain {
                return Err(format!(
                    "the trait method {} gives a `Result` or `Option`",
                    f.sig.ident
                ));
            }
            let rust = f.sig.ident.to_string();
            let go = if exported {
                names::exported(&rust)
            } else {
                names::unexported(&rust)
            };
            if let Some((other, _)) = methods.iter().find(|(_, (g, _, _))| *g == go) {
                return Err(format!(
                    "the trait methods {other} and {rust} are both {go} in Go"
                ));
            }
            methods.insert(rust, (go, params, results));
        }
        Ok(TraitInfo { exported, methods })
    }

    /// The signature of a function or method; `impl_ty` for one inside `impl impl_ty`.
    fn sig(
        &self,
        sig: &syn::Signature,
        public: bool,
        impl_ty: Option<&str>,
    ) -> Result<Sig, String> {
        let names = self.tparam_names(&sig.generics)?;
        let generic = !names.is_empty() || sig.generics.where_clause.is_some();
        let has_self = matches!(sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
        if generic && has_self {
            return Err(
                "a method with its own type parameters, which Go's methods cannot have".into(),
            );
        }
        // The parameter and result types with the names in scope, then the constraints: an
        // associated type in a type is refused as that, before its bound is.
        let outer = self.scope.borrow().clone();
        let mut scope = outer.clone();
        scope.params.extend(names.iter().map(|n| TypeParam {
            name: n.clone(),
            constraint: Type::Any,
        }));
        let mut s = self.in_scope(scope, || self.plain_sig(sig, public, impl_ty))?;
        if generic {
            s.tparams = self.constraints(&sig.generics, &names)?;
        }
        if !has_self && impl_ty.is_some() && !outer.params.is_empty() {
            let mut all = outer.params;
            all.append(&mut s.tparams);
            s.tparams = all;
        }
        Ok(s)
    }

    /// `sig` without its type parameters, which are in scope.
    fn plain_sig(
        &self,
        sig: &syn::Signature,
        public: bool,
        impl_ty: Option<&str>,
    ) -> Result<Sig, String> {
        if sig.asyncness.is_some() {
            return Err("an async function is outside cir/1".into());
        }
        if matches!(sig.safety, syn::Safety::Unsafe(_)) {
            return Err("an unsafe function is outside cir/1".into());
        }
        let mut recv = None;
        let mut params = Vec::new();
        for a in &sig.inputs {
            match a {
                syn::FnArg::Receiver(r) => {
                    let ty = impl_ty.ok_or("`self` outside an impl")?;
                    let named = Type::Named {
                        targs: self.scope.borrow().self_targs.clone(),
                        name: ty.into(),
                    };
                    recv = Some(match &r.kind {
                        syn::ReceiverKind::Value => named,
                        syn::ReceiverKind::Reference(..) => {
                            if self.ptr_types.contains(ty) {
                                Type::Ptr {
                                    elem: Box::new(named),
                                }
                            } else {
                                named
                            }
                        }
                        _ => return Err("a typed `self` receiver is outside cir/1".into()),
                    });
                }
                syn::FnArg::Typed(pt) => {
                    let t = self.ty(&pt.ty, impl_ty)?;
                    let rust = match &*pt.pat {
                        syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => {
                            i.ident.to_string()
                        }
                        syn::Pat::Wild(_) => "_".into(),
                        _ => return Err("a parameter pattern other than a name".into()),
                    };
                    let go = names::unexported(&rust);
                    if let Some((other, _, _)) =
                        params.iter().find(|(r, g, _)| *g == go && *r != "_")
                    {
                        return Err(format!(
                            "the parameters {other} and {rust} are both {go} in Go"
                        ));
                    }
                    params.push((rust.clone(), go, t));
                }
            }
        }
        let (results, shape) = self.results(&sig.output, impl_ty)?;
        let name = sig.ident.to_string();
        let mut go = if public {
            names::exported(&name)
        } else {
            names::unexported(&name)
        };
        if recv.is_none()
            && let Some(t) = impl_ty
        {
            go.push_str(t);
        }
        Ok(Sig {
            go,
            tparams: Vec::new(),
            recv,
            params,
            results,
            shape,
        })
    }

    /// Two items of one kind that `cir/1` would name alike (`value_2` and `value2` are both
    /// `value2`): two functions (associated ones among them), two methods of one type, two
    /// constants or two statics. `cir/1` keeps one declaration per name of each kind, so each of
    /// them is not declared and refused at its line, and each use of it refused; a method of a
    /// crate trait's impl refuses that whole impl. Items of different kinds (a type `Limit` and a
    /// function `limit`, a field and a method) are apart in `cir/1`; the Go writer spells them
    /// apart. Two fields refuse their struct and two parameters their function where they are
    /// read; locals are renamed apart as they are bound.
    fn go_name_clashes(
        &mut self,
        f: &syn::File,
        from_trait: &BTreeMap<(Option<String>, String), String>,
        deferred: &mut Vec<(proc_macro2::Span, String, String)>,
    ) {
        enum Item {
            Func((Option<String>, String)),
            Const(String),
            Static(String),
        }
        let what = |i: &Item| match i {
            Item::Func((None, n)) => format!("fn {n}"),
            Item::Func((Some(t), n)) => format!("fn {t}::{n}"),
            Item::Const(n) => format!("const {n}"),
            Item::Static(n) => format!("static {n}"),
        };
        // By kind (`fn`, `const`, `static`, or the type whose methods they are) and Go name.
        let mut groups: BTreeMap<(String, String), Vec<Item>> = BTreeMap::new();
        for (k, s) in &self.sigs {
            let kind = match (&k.0, &s.recv) {
                (Some(t), Some(_)) => format!("method of {t}"),
                _ => "fn".to_string(),
            };
            groups
                .entry((kind, s.go.clone()))
                .or_default()
                .push(Item::Func(k.clone()));
        }
        for (n, (go, _)) in &self.consts {
            groups
                .entry(("const".into(), go.clone()))
                .or_default()
                .push(Item::Const(n.clone()));
        }
        for (n, (go, _)) in &self.statics {
            groups
                .entry(("static".into(), go.clone()))
                .or_default()
                .push(Item::Static(n.clone()));
        }
        let span_of = |n: &str| {
            f.items.iter().find_map(|item| match item {
                syn::Item::Const(c) if c.ident == n => Some(c.span()),
                syn::Item::Static(s) if s.ident == n => Some(s.span()),
                _ => None,
            })
        };
        for ((_, go), items) in groups {
            if items.len() < 2 {
                continue;
            }
            let all: Vec<String> = items.iter().map(what).collect();
            let reason = format!(
                "{} are {} {go} in cir/1 and Go, which keep one of a kind per name",
                all.join(" and "),
                if all.len() == 2 { "both" } else { "all" }
            );
            for it in &items {
                match it {
                    Item::Func(k) => match (&k.0, from_trait.get(k)) {
                        (Some(t), Some(tr)) => {
                            self.trait_clashes
                                .insert((t.clone(), tr.clone()), reason.clone());
                            self.impls.remove(&(t.clone(), tr.clone()));
                            for (k2, tr2) in from_trait {
                                if k2.0 == k.0 && tr2 == tr {
                                    self.sigs.remove(k2);
                                }
                            }
                        }
                        _ => {
                            self.sigs.remove(k);
                            self.name_clashes.insert(k.clone(), reason.clone());
                        }
                    },
                    Item::Const(n) => {
                        self.consts.remove(n);
                        self.refused_consts.insert(n.clone(), reason.clone());
                        if let Some(sp) = span_of(n) {
                            deferred.push((sp, format!("const {n}"), reason.clone()));
                        }
                    }
                    Item::Static(n) => {
                        self.statics.remove(n);
                        self.refused_statics.insert(n.clone(), reason.clone());
                        if let Some(sp) = span_of(n) {
                            deferred.push((sp, format!("static {n}"), reason.clone()));
                        }
                    }
                }
            }
        }
    }

    /// Everything the bodies need before any is read: types, the error and message enums,
    /// receivers and signatures.
    fn scan(&mut self, f: &syn::File) {
        self.item_index = f
            .items
            .iter()
            .enumerate()
            .map(|(k, i)| (i as *const syn::Item as usize, k))
            .collect();
        for item in &f.items {
            self.enter(item);
            match item {
                syn::Item::Struct(s) => {
                    self.known.insert(s.ident.to_string());
                    if self.derives(&s.attrs, "std::default::Default") {
                        self.defaults.insert(s.ident.to_string());
                    }
                }
                syn::Item::Enum(e) => {
                    self.known.insert(e.ident.to_string());
                }
                syn::Item::Trait(t) => {
                    self.traits.insert(t.ident.to_string(), None);
                }
                _ => {}
            }
        }
        // A type a serde attribute gives a wire form of its own (wire names, a shape other than
        // the derived one) is refused naming the attribute: `cir/1` carries no wire form.
        let wire: BTreeMap<String, (proc_macro2::Span, String)> = f
            .items
            .iter()
            .filter_map(|item| {
                let (name, why) = match item {
                    syn::Item::Struct(s) => (s.ident.to_string(), serde_wire_struct(s)?),
                    syn::Item::Enum(e) => (e.ident.to_string(), serde_wire_enum(e)?),
                    _ => return None,
                };
                let why = format!(
                    "{why} sets the type's serde wire form, which cir/1 does not carry: the reader does not read a type whose wire names or shape a serde attribute changes"
                );
                Some((name, (item.span(), why)))
            })
            .collect();
        // Message enums and the error enum, by shape.
        for item in &f.items {
            self.enter(item);
            let syn::Item::Enum(e) = item else { continue };
            let name = e.ident.to_string();
            if wire.contains_key(&name) {
                continue;
            }
            if !e.generics.params.is_empty() {
                continue;
            }
            let wraps_own_name = !e.variants.is_empty()
                && e.variants.iter().all(|v| match &v.fields {
                    syn::Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                        self.own_type(&u.unnamed[0].ty)
                            .is_some_and(|t| v.ident == t)
                            && f.items
                                .iter()
                                .any(|i| matches!(i, syn::Item::Struct(s) if s.ident == v.ident))
                    }
                    _ => false,
                });
            if wraps_own_name && name != "Error" {
                self.message_enums.insert(name);
            }
        }
        // Struct, newtype, enum and union declarations, in order; a struct is resolved after every
        // name is known, so fields may name types declared later.
        let mut pending: Vec<&syn::Item> = Vec::new();
        for item in &f.items {
            self.enter(item);
            match item {
                syn::Item::Struct(s) if !wire.contains_key(&s.ident.to_string()) => {
                    pending.push(item)
                }
                syn::Item::Enum(e) if !wire.contains_key(&e.ident.to_string()) => {
                    pending.push(item)
                }
                _ => {}
            }
        }
        // Enums first (their decls do not depend on struct fields), then structs; refusals are
        // recorded once, at the end of `scan`, in source order.
        let mut deferred: Vec<(proc_macro2::Span, String, String)> = Vec::new();
        for (name, (span, why)) in &wire {
            deferred.push((*span, name.clone(), why.clone()));
        }
        for item in &pending {
            self.enter(item);
            if let syn::Item::Enum(e) = item {
                let name = e.ident.to_string();
                if self.message_enums.contains(&name) || (name == "Error") {
                    continue;
                }
                if !e.generics.params.is_empty() {
                    deferred.push((e.span(), name, "a generic enum is outside cir/1".into()));
                    continue;
                }
                if e.variants
                    .iter()
                    .all(|v| matches!(v.fields, syn::Fields::Unit))
                {
                    let variants: Vec<String> =
                        e.variants.iter().map(|v| v.ident.to_string()).collect();
                    self.decls.insert(
                        name,
                        Decl::Enum {
                            under: int(IntWidth::Platform, true),
                            variants,
                        },
                    );
                } else {
                    self.decls.insert(name, Decl::Union);
                }
            }
        }
        // Newtypes and structs may refer to each other: insert placeholders first.
        for item in &pending {
            self.enter(item);
            if let syn::Item::Struct(s) = item {
                self.decls
                    .insert(s.ident.to_string(), Decl::Struct(Vec::new()));
            }
        }
        // Newtypes before structs, so a struct field of a newtype type resolves.
        for item in &pending {
            self.enter(item);
            if let syn::Item::Struct(s) = item
                && let syn::Fields::Unnamed(u) = &s.fields
            {
                let name = s.ident.to_string();
                let under = if !s.generics.params.is_empty() {
                    Err("a generic tuple struct: the reader reads type parameters on a struct with named fields and on functions".into())
                } else if u.unnamed.len() == 1 {
                    self.ty(&u.unnamed[0].ty, Some(&name))
                } else {
                    Err("a tuple struct of more than one field is outside cir/1".into())
                };
                match under {
                    Ok(t @ (Type::Int { .. } | Type::Bool | Type::String)) => {
                        self.decls.insert(name, Decl::Newtype(t));
                    }
                    Ok(_) => {
                        self.decls.remove(&name);
                        deferred.push((
                            s.span(),
                            name,
                            "a tuple struct over a type other than an integer, bool or String"
                                .into(),
                        ));
                    }
                    Err(e) => {
                        self.decls.remove(&name);
                        deferred.push((s.span(), name, e));
                    }
                }
            }
        }
        // The error enum, now that every struct name is declared.
        for item in &f.items {
            self.enter(item);
            let syn::Item::Enum(e) = item else { continue };
            if e.ident != "Error" || wire.contains_key("Error") {
                continue;
            }
            let mut vs = BTreeMap::new();
            let mut bad = None;
            for v in &e.variants {
                let vn = v.ident.to_string();
                match &v.fields {
                    syn::Fields::Unit => {
                        vs.insert(
                            vn.clone(),
                            ErrVariant::Sentinel {
                                go: names::sentinel(&vn),
                            },
                        );
                    }
                    syn::Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                        let ty = &u.unnamed[0].ty;
                        let string = self
                            .std_type(ty)
                            .is_some_and(|(s, a)| s == "std::string::String" && a.is_empty());
                        match self.own_type(ty) {
                            _ if string => {
                                vs.insert(vn, ErrVariant::Msg);
                            }
                            Some(t) if matches!(self.decls.get(&t), Some(Decl::Struct(_))) => {
                                vs.insert(vn, ErrVariant::Struct(t));
                            }
                            _ => {
                                bad = Some(format!(
                                    "variant {vn} wraps neither a struct nor a String"
                                ))
                            }
                        }
                    }
                    _ => {
                        bad = Some(format!(
                            "variant {vn} has fields other than one struct or String"
                        ))
                    }
                }
            }
            if !e.generics.params.is_empty() {
                bad = Some("a generic enum Error".into());
            }
            match bad {
                Some(b) => deferred.push((e.span(), "Error".into(), format!("enum Error: {b}"))),
                None => {
                    for v in vs.values() {
                        if let ErrVariant::Struct(s) = v {
                            self.error_structs.insert(s.clone());
                        }
                    }
                    self.error_enum = Some(("Error".into(), vs));
                }
            }
        }
        // Traits, once every type name is declared: each method's signature. A trait that does
        // not read is forgotten, so no `dyn` of it reads either.
        for item in &f.items {
            self.enter(item);
            let syn::Item::Trait(t) = item else { continue };
            let name = t.ident.to_string();
            match self.trait_info(t) {
                Ok(info) => {
                    self.traits.insert(name, Some(info));
                }
                Err(e) => {
                    self.traits.remove(&name);
                    deferred.push((t.span(), name, e));
                }
            }
        }
        // The type parameters of generic structs, before any field names one: each constrained
        // by `any` or a trait of the crate.
        for item in &pending {
            self.enter(item);
            if let syn::Item::Struct(s) = item
                && !matches!(s.fields, syn::Fields::Unnamed(_))
                && (!s.generics.params.is_empty() || s.generics.where_clause.is_some())
            {
                let name = s.ident.to_string();
                match self
                    .tparam_names(&s.generics)
                    .and_then(|n| self.constraints(&s.generics, &n))
                {
                    Ok(tps) => {
                        self.type_params.insert(name, tps);
                    }
                    Err(e) => {
                        self.decls.remove(&name);
                        deferred.push((s.span(), name, e));
                    }
                }
            }
        }
        // Struct fields.
        for item in &pending {
            self.enter(item);
            if let syn::Item::Struct(s) = item {
                let name = s.ident.to_string();
                if matches!(s.fields, syn::Fields::Unnamed(_)) || !self.decls.contains_key(&name) {
                    continue;
                }
                let scope = match self.type_params.get(&name) {
                    Some(tps) => Scope {
                        self_targs: tps
                            .iter()
                            .map(|p| Type::Param {
                                name: p.name.clone(),
                            })
                            .collect(),
                        params: tps.clone(),
                    },
                    None => Scope::default(),
                };
                let mut fields = Vec::new();
                let mut bad = None;
                if let syn::Fields::Named(n) = &s.fields {
                    for fl in &n.named {
                        let rust = fl.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                        match self.in_scope(scope.clone(), || self.ty(&fl.ty, Some(&name))) {
                            Ok(t) => {
                                let go = if is_pub(&fl.vis) {
                                    names::exported(&rust)
                                } else {
                                    names::unexported(&rust)
                                };
                                if let Some(k) = body::map_kind(self, &fl.ty) {
                                    self.map_fields.insert((name.clone(), go.clone()), k);
                                }
                                fields.push(FieldInfo { go, rust, t })
                            }
                            Err(e) => bad = Some(format!("field {rust}: {e}")),
                        }
                    }
                }
                // Two fields one Go name (`value_2`, `value2`) would be one Go field.
                for (i, a) in fields.iter().enumerate() {
                    if let Some(b) = fields[..i].iter().find(|b| b.go == a.go) {
                        bad = Some(format!(
                            "the fields {} and {} are both {} in Go",
                            b.rust, a.rust, a.go
                        ));
                    }
                }
                match bad {
                    Some(b) => {
                        self.decls.remove(&name);
                        deferred.push((s.span(), name, b));
                    }
                    None => {
                        self.decls.insert(name, Decl::Struct(fields));
                    }
                }
            }
        }
        // A struct read before a type it names was refused (a field of a struct declared later)
        // names a type the module does not declare: it is refused in turn, until none does.
        loop {
            let dangling: Vec<(String, String)> = self
                .decls
                .iter()
                .filter_map(|(n, d)| match d {
                    Decl::Struct(fields) => fields.iter().find_map(|fl| {
                        self.undeclared(&fl.t).map(|u| {
                            (
                                n.clone(),
                                format!("field {}: type {u}, which is not read", fl.rust),
                            )
                        })
                    }),
                    Decl::Newtype(t) => self
                        .undeclared(t)
                        .map(|u| (n.clone(), format!("type {u}, which is not read"))),
                    _ => None,
                })
                .collect();
            if dangling.is_empty() {
                break;
            }
            for (n, why) in dangling {
                self.decls.remove(&n);
                if let Some(s) = pending.iter().find_map(|i| match i {
                    syn::Item::Struct(s) if s.ident == n => Some(s),
                    _ => None,
                }) {
                    deferred.push((s.span(), n, why));
                }
            }
        }
        self.type_params.retain(|n, _| self.decls.contains_key(n));
        // `#[derive(Default)]` makes Go's zero value only when every field's default is its
        // zero: an integer, float, `bool`, `String`, collection or array of such, or a type of
        // the crate whose derived default is its zero in turn. An `Rc<RefCell<T>>` field's default
        // is a new value where Go's zero pointer is nil, an enum's is the variant `#[default]`
        // names, whatever its value.
        loop {
            let before = self.defaults.len();
            let keep: BTreeSet<String> = self
                .defaults
                .iter()
                .filter(|n| match self.decls.get(*n) {
                    Some(Decl::Struct(fs)) => fs.iter().all(|f| self.zero_default(&f.t)),
                    Some(Decl::Newtype(u)) => self.zero_default(u),
                    _ => false,
                })
                .cloned()
                .collect();
            self.defaults = keep;
            if self.defaults.len() == before {
                break;
            }
        }
        // `impl Trait for T` of a crate trait.
        for item in &f.items {
            self.enter(item);
            if let syn::Item::Impl(im) = item
                && let Some((p, _)) = &im.trait_
                && let Some(tr) = self.own_item(&self.full_path(p))
                && matches!(self.traits.get(&tr), Some(Some(_)))
            {
                match &*im.self_ty {
                    // `impl Tr for &T`: kept apart, since the reader reads `&T` as `T`.
                    syn::Type::Reference(r) => {
                        if let Some(t) = self.own_type(&r.elem) {
                            self.ref_impls.insert((t, tr));
                        }
                    }
                    t => {
                        if let Some(t) = self.own_type(t).filter(|t| self.decls.contains_key(t)) {
                            self.impls.insert((t, tr));
                        }
                    }
                }
            }
        }
        // Two crate traits implemented by one type with a method of one name: Go has one method
        // per name, so neither impl reads.
        let mut by_method: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        for (t, tr) in &self.impls {
            if let Some(Some(info)) = self.traits.get(tr) {
                for m in info.methods.keys() {
                    by_method
                        .entry((t.clone(), m.clone()))
                        .or_default()
                        .push(tr.clone());
                }
            }
        }
        for ((t, m), trs) in by_method {
            if trs.len() > 1 {
                for tr in &trs {
                    self.trait_clashes.insert(
                        (t.clone(), tr.clone()),
                        format!(
                            "{t} implements {} with a method {m} each: Go has one method per name, so neither reads",
                            trs.join(" and ")
                        ),
                    );
                }
            }
        }
        for key in self.trait_clashes.keys() {
            self.impls.remove(key);
        }
        // Receivers: a type with any `&mut self` method has pointer receivers throughout.
        for item in &f.items {
            self.enter(item);
            let syn::Item::Impl(im) = item else { continue };
            if im.trait_.is_some() {
                continue;
            }
            let Some(t) = last_seg(&im.self_ty).map(|(i, _)| i.to_string()) else {
                continue;
            };
            for ii in &im.items {
                if let syn::ImplItem::Fn(func) = ii
                    && let Some(syn::FnArg::Receiver(r)) = func.sig.inputs.first()
                    && matches!(r.kind, syn::ReceiverKind::Reference(_, _, Some(_)))
                {
                    self.ptr_types.insert(t.clone());
                }
            }
        }
        // Value enums.
        for item in &f.items {
            self.enter(item);
            let syn::Item::Impl(im) = item else { continue };
            if im.trait_.is_some() {
                continue;
            }
            let Some(t) = plain_ident(&im.self_ty) else {
                continue;
            };
            let Some(Decl::Enum { variants, .. }) = self.decls.get(&t).cloned() else {
                continue;
            };
            for ii in &im.items {
                if let syn::ImplItem::Fn(func) = ii
                    && let Some((under, _)) = value_idiom(func, &t, &variants)
                {
                    self.decls.insert(
                        t.clone(),
                        Decl::Enum {
                            under,
                            variants: variants.clone(),
                        },
                    );
                    self.value_enums.insert(t.clone());
                }
            }
        }
        // Display impls: which types render. An impl of a trait named `Display` that is not
        // std's (`impl other::Display for T`) gives `{}` and `to_string()` of `T` no reading.
        for item in &f.items {
            self.enter(item);
            if let syn::Item::Impl(im) = item
                && let Some((p, _)) = &im.trait_
                && let Some(t) = plain_ident(&im.self_ty)
            {
                if self.std_path(p) == Some("std::fmt::Display") {
                    self.displays.insert(t);
                } else if p.segments.last().is_some_and(|s| s.ident == "Display") {
                    let why = format!(
                        "its impl of `{}` is not std's Display (`{}`)",
                        src_of(p),
                        self.full_path(p).join("::")
                    );
                    self.refused_displays.insert(t, why);
                }
            }
        }
        // Comparison impls written by hand: `==`, `<` and the rest on the type call them, which
        // `cir/1`'s built-in comparison is not. A trait of that name the crate declares is not
        // std's, unless named by a std path.
        for item in &f.items {
            self.enter(item);
            if let syn::Item::Impl(im) = item
                && let Some((p, _)) = &im.trait_
                && let Some(tr) = p.segments.last().map(|s| s.ident.to_string())
                && (!self.traits.contains_key(&tr)
                    || matches!(
                        self.full_path(p).first().map(String::as_str),
                        Some("std" | "core")
                    ))
                && let Some((t, _)) = last_seg(match &*im.self_ty {
                    syn::Type::Reference(r) => &r.elem,
                    t => t,
                })
            {
                let t = t.to_string();
                match tr.as_str() {
                    "PartialEq" => {
                        self.hand_eq.insert(t);
                    }
                    "PartialOrd" | "Ord" => {
                        self.hand_ord.insert(t);
                    }
                    _ => {}
                }
            }
        }
        // Constants.
        for item in &f.items {
            self.enter(item);
            if let syn::Item::Const(c) = item {
                let rust = c.ident.to_string();
                let go = if is_pub(&c.vis) {
                    names::exported(&screaming(&rust))
                } else {
                    names::unexported(&screaming(&rust))
                };
                // A constant whose type or value is refused is not declared, and each use of it
                // is refused naming it.
                let read = self
                    .ty(&c.ty, None)
                    .and_then(|t| body::const_value(self, &c.expr, &t, "constant").map(|v| (t, v)));
                match read {
                    Ok((t, v)) => {
                        if let Expr::Lit { value, .. } = v {
                            self.const_values.insert(go.clone(), value);
                        }
                        self.consts.insert(rust, (go, t));
                    }
                    Err(e) => {
                        self.refused_consts.insert(rust.clone(), e.clone());
                        deferred.push((c.span(), format!("const {rust}"), e));
                    }
                }
            }
        }
        // Statics: Go's module-level variables. A `static mut` is read and written only inside
        // `unsafe`, without synchronisation, as Go's variable is.
        for item in &f.items {
            self.enter(item);
            if let syn::Item::Static(s) = item {
                let rust = s.ident.to_string();
                let go = if is_pub(&s.vis) {
                    names::exported(&screaming(&rust))
                } else {
                    names::unexported(&screaming(&rust))
                };
                // A static whose type or initialiser is refused is not declared, and each use of
                // it is refused naming it.
                let read = self
                    .ty(&s.ty, None)
                    .and_then(|t| body::const_value(self, &s.expr, &t, "static").map(|_| t));
                match read {
                    Ok(t) => {
                        self.statics.insert(rust, (go, t));
                    }
                    Err(e) => {
                        self.refused_statics.insert(rust.clone(), e.clone());
                        deferred.push((s.span(), format!("static {rust}"), e));
                    }
                }
            }
        }
        // Signatures; the methods of a crate trait's impl by the trait.
        let mut from_trait: BTreeMap<(Option<String>, String), String> = BTreeMap::new();
        for item in &f.items {
            self.enter(item);
            match item {
                syn::Item::Fn(func) => {
                    if let Ok(s) = self.sig(&func.sig, is_pub(&func.vis), None) {
                        self.sigs.insert((None, func.sig.ident.to_string()), s);
                    }
                }
                syn::Item::Impl(im) => {
                    let Ok((t, scope)) = self.impl_scope(im) else {
                        continue;
                    };
                    // The methods of a crate trait's impl are the type's methods, named as the
                    // trait names them.
                    let public = match &im.trait_ {
                        None => None,
                        Some((p, _)) => {
                            let tr = self.own_item(&self.full_path(p)).unwrap_or_default();
                            if self.trait_clashes.contains_key(&(t.clone(), tr.clone())) {
                                continue;
                            }
                            match self.traits.get(&tr) {
                                Some(Some(info)) => Some(info.exported),
                                _ => continue,
                            }
                        }
                    };
                    for ii in &im.items {
                        if let syn::ImplItem::Fn(func) = ii
                            && (public.is_none() || !inherent_method(f, &t, &func.sig.ident))
                            && let Ok(s) = self.in_scope(scope.clone(), || {
                                self.sig(
                                    &func.sig,
                                    public.unwrap_or_else(|| is_pub(&func.vis)),
                                    Some(&t),
                                )
                            })
                        {
                            let key = (Some(t.clone()), func.sig.ident.to_string());
                            if let Some((p, _)) = &im.trait_ {
                                from_trait.insert(
                                    key.clone(),
                                    self.own_item(&self.full_path(p)).unwrap_or_default(),
                                );
                            }
                            self.sigs.insert(key, s);
                        }
                    }
                }
                _ => {}
            }
        }
        self.go_name_clashes(f, &from_trait, &mut deferred);
        // A type whose `Display` impl is refused does not render: each `{}` and `to_string()` of
        // it is refused at its line. One impl's rendering may format another type, so this runs
        // until no more are refused.
        loop {
            let refused: Vec<(String, String)> = f
                .items
                .iter()
                .filter_map(|item| match item {
                    syn::Item::Impl(im)
                        if im.trait_.as_ref().is_some_and(|(p, _)| {
                            self.enter(item);
                            self.std_path(p) == Some("std::fmt::Display")
                        }) =>
                    {
                        let t = plain_ident(&im.self_ty)?;
                        if !self.displays.contains(&t)
                            || self.error_enum.as_ref().is_some_and(|(e, _)| *e == t)
                            || self.message_enums.contains(&t)
                        {
                            return None;
                        }
                        body::display(self, &t, im).err().map(|why| (t, why))
                    }
                    _ => None,
                })
                .collect();
            if refused.is_empty() {
                break;
            }
            for (t, why) in refused {
                self.displays.remove(&t);
                self.refused_displays.insert(t, why);
            }
        }
        deferred.sort_by_key(|(span, _, _)| self.pos_key(&self.pos(*span)));
        for (span, what, reason) in deferred {
            self.refuse(span, &what, reason);
        }
    }

    /// The module, items in source order.
    fn module(mut self, name: &str, f: &syn::File) -> Module {
        let mut types = Vec::new();
        let mut errors = Vec::new();
        let mut funcs = Vec::new();
        let mut consts = Vec::new();
        let mut layout = Vec::new();
        let mut vars = Vec::new();
        let mut interfaces = Vec::new();
        let mut late_refusals: Vec<Refusal> = Vec::new();
        for (k, item) in f.items.iter().enumerate() {
            let module = self
                .item_modules
                .get(k)
                .cloned()
                .unwrap_or_else(|| "crate".into());
            let self_ty = match item {
                syn::Item::Impl(im) => impl_type_name(&im.self_ty),
                _ => None,
            };
            *self.at.borrow_mut() = (module, self_ty);
            self.spans.borrow_mut().clear();
            match item {
                syn::Item::Struct(s) => {
                    let n = s.ident.to_string();
                    let pos = self.pos(s.span());
                    let d = doc(&s.attrs);
                    let exported = is_pub(&s.vis);
                    match self.decls.get(&n) {
                        Some(Decl::Struct(fields)) => {
                            self.note_fields(types.len(), s, fields);
                            types.push(TypeDecl::Struct {
                                tparams: self.type_params.get(&n).cloned().unwrap_or_default(),
                                name: n.clone(),
                                exported,
                                fields: fields
                                    .iter()
                                    .map(|fi| Field {
                                        name: fi.go.clone(),
                                        t: fi.t.clone(),
                                        embedded: false,
                                    })
                                    .collect(),
                                doc: d,
                                pos,
                            })
                        }
                        Some(Decl::Newtype(under)) => types.push(TypeDecl::Newtype {
                            tparams: Vec::new(),
                            name: n.clone(),
                            exported,
                            under: under.clone(),
                            doc: d,
                            pos,
                        }),
                        _ => continue,
                    }
                    layout.push(format!("type:{n}"));
                }
                syn::Item::Enum(e) => {
                    let n = e.ident.to_string();
                    if self.message_enums.contains(&n) {
                        continue;
                    }
                    if n == "Error" {
                        if let Some(es) = self.error_enum_decl(e, f, &mut late_refusals) {
                            for ev in es {
                                layout.push(format!("error:{}", ev.name));
                                errors.push(ev);
                            }
                        }
                        continue;
                    }
                    let pos = self.pos(e.span());
                    let d = doc(&e.attrs);
                    let exported = is_pub(&e.vis);
                    match self.decls.get(&n).cloned() {
                        Some(Decl::Enum { under, .. }) => {
                            let values = if self.value_enums.contains(&n) {
                                value_from_impls(
                                    f,
                                    &n,
                                    &e.variants
                                        .iter()
                                        .map(|v| v.ident.to_string())
                                        .collect::<Vec<_>>(),
                                )
                            } else {
                                discriminants(e)
                            };
                            match values {
                                Ok(values) => types.push(TypeDecl::Enum {
                                    name: n.clone(),
                                    exported,
                                    under,
                                    variants: e
                                        .variants
                                        .iter()
                                        .zip(values)
                                        .map(|(v, value)| Variant {
                                            name: v.ident.to_string(),
                                            value,
                                        })
                                        .collect(),
                                    doc: d,
                                    pos,
                                }),
                                Err(r) => {
                                    late_refusals.push(Refusal {
                                        pos,
                                        what: n,
                                        reason: r,
                                    });
                                    continue;
                                }
                            }
                        }
                        Some(Decl::Union) => {
                            let mut variants = Vec::new();
                            let mut bad = None;
                            for v in &e.variants {
                                let mut fields = Vec::new();
                                for (i, fl) in v.fields.iter().enumerate() {
                                    // `_<digits>` marks a tuple field in cir/1: a struct
                                    // variant's field of that name would read as one.
                                    let fname = match &fl.ident {
                                        Some(id) if tuple_field_name(&id.to_string()) => {
                                            bad = Some((
                                                self.pos(fl.span()),
                                                format!(
                                                    "the field `{id}` of the struct variant {}: a name `_<digits>` marks a tuple field in cir/1, so the variant would read as the tuple variant `{}(…)`",
                                                    v.ident, v.ident
                                                ),
                                            ));
                                            continue;
                                        }
                                        Some(id) => names::exported(&id.to_string()),
                                        None => format!("_{i}"),
                                    };
                                    match self.ty(&fl.ty, Some(&n)) {
                                        Ok(t) => fields.push(Field {
                                            name: fname,
                                            t,
                                            embedded: false,
                                        }),
                                        Err(r) => bad = Some((pos.clone(), r)),
                                    }
                                }
                                variants.push(UnionVariant {
                                    name: v.ident.to_string(),
                                    fields,
                                });
                            }
                            if let Some((pos, r)) = bad {
                                late_refusals.push(Refusal {
                                    pos,
                                    what: n,
                                    reason: r,
                                });
                                continue;
                            }
                            types.push(TypeDecl::Union {
                                name: n.clone(),
                                exported,
                                variants,
                                doc: d,
                                pos,
                            });
                        }
                        _ => continue,
                    }
                    layout.push(format!("type:{n}"));
                }
                syn::Item::Fn(func) => {
                    let key = (None, func.sig.ident.to_string());
                    match self.sigs.get(&key).cloned() {
                        Some(sig) => {
                            let (fu, rs) = self.in_scope(Scope::of(sig.tparams.clone()), || {
                                body::func(
                                    &self,
                                    &sig,
                                    &func.sig,
                                    &func.block,
                                    None,
                                    doc(&func.attrs),
                                    self.pos(func.sig.ident.span()),
                                )
                            });
                            late_refusals.extend(rs);
                            layout.push(format!("func:{}", fu.key()));
                            self.spans.borrow_mut().func(
                                funcs.len(),
                                func.span(),
                                Ok(func.sig.ident.span()),
                            );
                            funcs.push(fu);
                        }
                        None => {
                            let reason = match self.name_clashes.get(&key) {
                                Some(why) => why.clone(),
                                None => self
                                    .sig(&func.sig, is_pub(&func.vis), None)
                                    .err()
                                    .unwrap_or_default(),
                            };
                            self.note_opaque(func.span(), &func.sig);
                            self.push_opaque(None, &func.sig, &func.block, &reason);
                            late_refusals.push(Refusal {
                                pos: self.pos(func.sig.ident.span()),
                                what: format!("fn {}", func.sig.ident),
                                reason,
                            });
                        }
                    }
                }
                syn::Item::Impl(im) => {
                    let (t, scope) = match self.impl_scope(im) {
                        Ok(ts) => ts,
                        Err(reason) => {
                            self.push_opaque_impl(im, &reason);
                            late_refusals.push(Refusal {
                                pos: self.pos(im.span()),
                                what: format!("impl {}", src_of(&im.self_ty)),
                                reason,
                            });
                            continue;
                        }
                    };
                    let crate_trait = im.trait_.as_ref().and_then(|(p, _)| {
                        let n = self.own_item(&self.full_path(p))?;
                        matches!(self.traits.get(&n), Some(Some(_))).then_some(n)
                    });
                    if let Some(tr) = &crate_trait
                        && let Some(reason) = self.trait_clashes.get(&(t.clone(), tr.clone()))
                    {
                        let reason = reason.clone();
                        self.push_opaque_impl(im, &reason);
                        late_refusals.push(Refusal {
                            pos: self.pos(im.span()),
                            what: format!("impl {tr} for {t}"),
                            reason,
                        });
                        continue;
                    }
                    if let Some(tr) = &crate_trait
                        && let Some(clash) = im.items.iter().find_map(|ii| match ii {
                            syn::ImplItem::Fn(func) if inherent_method(f, &t, &func.sig.ident) => {
                                Some(func.sig.ident.to_string())
                            }
                            _ => None,
                        })
                    {
                        let reason = format!(
                            "{t} has an inherent method {clash} too: Go has one method per name"
                        );
                        self.push_opaque_impl(im, &reason);
                        late_refusals.push(Refusal {
                            pos: self.pos(im.span()),
                            what: format!("impl {tr} for {t}"),
                            reason,
                        });
                        continue;
                    }
                    match &im.trait_ {
                        _ if im.trait_.is_none() || crate_trait.is_some() => {
                            for ii in &im.items {
                                let syn::ImplItem::Fn(func) = ii else {
                                    late_refusals.push(Refusal {
                                        pos: self.pos(ii.span()),
                                        what: format!("impl {t}"),
                                        reason: "an impl item other than a function".into(),
                                    });
                                    continue;
                                };
                                if self.value_enums.contains(&t) && func.sig.ident == "value" {
                                    continue;
                                }
                                let key = (Some(t.clone()), func.sig.ident.to_string());
                                match self.sigs.get(&key).cloned() {
                                    Some(sig) => {
                                        // A method's type parameters are its receiver's; a
                                        // function's its own, the impl's first.
                                        let body_scope = Scope {
                                            params: if sig.recv.is_some() {
                                                scope.params.clone()
                                            } else {
                                                sig.tparams.clone()
                                            },
                                            self_targs: scope.self_targs.clone(),
                                        };
                                        let (fu, rs) = self.in_scope(body_scope, || {
                                            body::func(
                                                &self,
                                                &sig,
                                                &func.sig,
                                                &func.block,
                                                Some(&t),
                                                doc(&func.attrs),
                                                self.pos(func.sig.ident.span()),
                                            )
                                        });
                                        late_refusals.extend(rs);
                                        layout.push(format!("func:{}", fu.key()));
                                        self.spans.borrow_mut().func(
                                            funcs.len(),
                                            func.span(),
                                            Ok(func.sig.ident.span()),
                                        );
                                        funcs.push(fu);
                                    }
                                    None => {
                                        let reason = match self.name_clashes.get(&key) {
                                            Some(why) => why.clone(),
                                            None => self
                                                .in_scope(scope.clone(), || {
                                                    self.sig(&func.sig, is_pub(&func.vis), Some(&t))
                                                })
                                                .err()
                                                .unwrap_or_default(),
                                        };
                                        self.note_opaque(func.span(), &func.sig);
                                        self.push_opaque(Some(&t), &func.sig, &func.block, &reason);
                                        late_refusals.push(Refusal {
                                            pos: self.pos(func.sig.ident.span()),
                                            what: format!("fn {t}::{}", func.sig.ident),
                                            reason,
                                        });
                                    }
                                }
                            }
                        }
                        Some((p, _)) => {
                            let tr = p
                                .segments
                                .last()
                                .map(|s| s.ident.to_string())
                                .unwrap_or_default();
                            let std = self.std_path(p);
                            if std == Some("std::error::Error") && im.items.is_empty() {
                                continue;
                            }
                            if std == Some("std::fmt::Display") {
                                if self.error_enum.as_ref().is_some_and(|(e, _)| *e == t)
                                    || self.message_enums.contains(&t)
                                {
                                    continue;
                                }
                                match body::display(&self, &t, im) {
                                    Ok(Some((fu, rs))) => {
                                        late_refusals.extend(rs);
                                        layout.push(format!("func:{}", fu.key()));
                                        self.spans.borrow_mut().func(
                                            funcs.len(),
                                            im.span(),
                                            Err(format!(
                                                "the method {} is the impl's fmt, no token spells it",
                                                fu.name
                                            )),
                                        );
                                        funcs.push(fu);
                                    }
                                    Ok(None) => {}
                                    Err(reason) => {
                                        self.push_opaque_impl(im, &reason);
                                        late_refusals.push(Refusal {
                                            pos: self.pos(im.span()),
                                            what: format!("impl Display for {t}"),
                                            reason,
                                        })
                                    }
                                }
                                continue;
                            }
                            let reason = format!(
                                "an impl of {tr}, which is not a trait of the crate, is outside cir/1"
                            );
                            self.push_opaque_impl(im, &reason);
                            late_refusals.push(Refusal {
                                pos: self.pos(im.span()),
                                what: format!("impl {tr} for {t}"),
                                reason,
                            });
                        }
                        None => unreachable!("an inherent impl is read by the arm above"),
                    }
                }
                syn::Item::Const(c) => {
                    let rust = c.ident.to_string();
                    let Some((go, t)) = self.consts.get(&rust).cloned() else {
                        continue;
                    };
                    match body::const_value(&self, &c.expr, &t, "constant") {
                        Ok(value) => {
                            layout.push(format!("const:{go}"));
                            consts.push(ConstDecl {
                                name: go,
                                t,
                                value,
                                pos: self.pos(c.span()),
                            });
                        }
                        Err(reason) => late_refusals.push(Refusal {
                            pos: self.pos(c.span()),
                            what: format!("const {rust}"),
                            reason,
                        }),
                    }
                }
                syn::Item::Static(s) => {
                    let rust = s.ident.to_string();
                    let Some((go, t)) = self.statics.get(&rust).cloned() else {
                        continue;
                    };
                    match body::const_value(&self, &s.expr, &t, "static") {
                        Ok(value) => {
                            layout.push(format!("var:{go}"));
                            let zero = matches!(&value, Expr::Lit { value, .. } if value == "0" || value == "false" || value == "\"\"")
                                || matches!(&value, Expr::Float { value, .. } if value == "0");
                            vars.push(VarDecl {
                                name: go,
                                t,
                                init: (!zero).then_some(value),
                                foreign: None,
                                pos: self.pos(s.span()),
                            });
                        }
                        Err(reason) => late_refusals.push(Refusal {
                            pos: self.pos(s.span()),
                            what: format!("static {rust}"),
                            reason,
                        }),
                    }
                }
                syn::Item::Trait(tr) => {
                    let name = tr.ident.to_string();
                    let Some(Some(info)) = self.traits.get(&name) else {
                        continue;
                    };
                    let mut methods: Vec<IfaceMethod> = info
                        .methods
                        .values()
                        .map(|(go, params, results)| IfaceMethod {
                            name: go.clone(),
                            params: params.clone(),
                            results: results.clone(),
                        })
                        .collect();
                    methods.sort_by(|a, b| a.name.cmp(&b.name));
                    layout.push(format!("type:{name}"));
                    interfaces.push(Interface {
                        path: String::new(),
                        name,
                        exported: info.exported,
                        methods,
                        doc: doc(&tr.attrs),
                        pos: self.pos(tr.span()),
                    });
                }
                syn::Item::Use(u) => {
                    if self.noting() {
                        let module = self.at.borrow().0.clone();
                        self.note_use(&module, &u.tree, &mut Vec::new());
                    }
                }
                other => {
                    let what = first_line(&src_of(other));
                    late_refusals.push(Refusal {
                        pos: self.pos(other.span()),
                        what,
                        reason: "an item other than a struct, enum, impl, fn, const or use is outside cir/1".into(),
                    });
                }
            }
        }
        let mut refusals = std::mem::take(&mut self.refusals);
        refusals.extend(late_refusals);
        for r in &mut refusals {
            r.what = one_line(&r.what);
            r.reason = one_line(&r.reason);
        }
        refusals.sort_by_key(|r| self.pos_key(&r.pos));
        self.push_apart();
        let mut opaque = self.opaque_named_apart(&funcs);
        for o in &mut opaque {
            o.reason = one_line(&o.reason);
        }
        Module {
            format: codespec_model::FORMAT.into(),
            name: name.into(),
            reader: ReaderInfo {
                language: "rust".into(),
                name: "codespec-read-rust".into(),
                version: VERSION.into(),
            },
            doc: doc(&f.attrs),
            types,
            consts,
            errors,
            vars,
            funcs,
            externs: vec![],
            layout,
            facts: None,
            refusals,
            path: String::new(),
            opaque,
            methods: vec![],
            linknames: vec![],
            inits: vec![],
            interfaces,
            entries: vec![],
            beans: vec![],
            injects: vec![],
            transactional: vec![],
            persistence: vec![],
        }
    }

    /// A function refused whole, kept as an opaque function with the calls of its body: named
    /// as the crate names it in `cir/1` (`ty` the impl's type, for a function of an impl).
    fn push_opaque(
        &mut self,
        ty: Option<&str>,
        sig: &syn::Signature,
        block: &syn::Block,
        reason: &str,
    ) {
        let rust = sig.ident.to_string();
        let (recv, name) = match ty {
            None => (
                None,
                self.tables
                    .fns
                    .get(&rust)
                    .cloned()
                    .unwrap_or_else(|| calls::fn_name(&rust, false)),
            ),
            Some(t) => self
                .tables
                .assoc
                .get(&(t.to_string(), rust.clone()))
                .cloned()
                .unwrap_or_else(|| (Some(t.to_string()), calls::fn_name(&rust, false))),
        };
        let calls = calls::of_block(self, block);
        self.opaque.push(codespec_model::Opaque {
            name,
            recv,
            pos: self.pos(sig.ident.span()),
            reason: reason.to_string(),
            no_body: false,
            calls,
        });
    }

    /// The functions and impls refused for a name two modules declare: each function an opaque
    /// one, named as the reader names a function it reads, listing its calls.
    fn push_apart(&mut self) {
        for (item, module, reason) in std::mem::take(&mut self.apart) {
            let self_ty = match &item {
                syn::Item::Impl(im) => impl_type_name(&im.self_ty),
                _ => None,
            };
            *self.at.borrow_mut() = (module, self_ty.clone());
            let push = |r: &mut Self,
                        recv: Option<String>,
                        name: String,
                        f: (&syn::Signature, &syn::Block)| {
                let calls = calls::of_block(r, f.1);
                r.opaque.push(codespec_model::Opaque {
                    name,
                    recv,
                    pos: r.pos(f.0.ident.span()),
                    reason: reason.clone(),
                    no_body: false,
                    calls,
                });
            };
            match &item {
                syn::Item::Fn(f) => {
                    let name = calls::fn_name(&f.sig.ident.to_string(), is_pub(&f.vis));
                    self.note_opaque(f.span(), &f.sig);
                    push(self, None, name, (&f.sig, &f.block));
                }
                syn::Item::Impl(im) => {
                    let Some(t) = self_ty else { continue };
                    for ii in &im.items {
                        let syn::ImplItem::Fn(f) = ii else { continue };
                        let base = calls::fn_name(&f.sig.ident.to_string(), is_pub(&f.vis));
                        self.note_opaque(f.span(), &f.sig);
                        if matches!(f.sig.inputs.first(), Some(syn::FnArg::Receiver(_))) {
                            push(self, Some(t.clone()), base, (&f.sig, &f.block));
                        } else {
                            push(self, None, format!("{base}{t}"), (&f.sig, &f.block));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Every function of an impl refused whole, each an opaque function.
    fn push_opaque_impl(&mut self, im: &syn::ItemImpl, reason: &str) {
        let Some(t) = impl_type_name(&im.self_ty) else {
            return;
        };
        for ii in &im.items {
            if let syn::ImplItem::Fn(func) = ii {
                self.note_opaque(func.span(), &func.sig);
                self.push_opaque(Some(&t), &func.sig, &func.block, reason);
            }
        }
    }

    /// Note for `--spans` the next opaque function: its whole item and its identifier.
    fn note_opaque(&self, decl: proc_macro2::Span, sig: &syn::Signature) {
        self.spans
            .borrow_mut()
            .opaque(self.opaque.len(), decl, sig.ident.span());
    }

    /// Note for `--spans` the identifier of each field of the struct declaration `s`, read as the
    /// struct at `index` of `types` with `fields`.
    fn note_fields(&self, index: usize, s: &syn::ItemStruct, fields: &[FieldInfo]) {
        if !self.noting() {
            return;
        }
        for (j, fi) in fields.iter().enumerate() {
            if let Some(id) = s
                .fields
                .iter()
                .filter_map(|f| f.ident.as_ref())
                .find(|id| **id == fi.rust)
            {
                self.spans.borrow_mut().field(index, j, id.span());
            }
        }
    }

    /// The opaque functions, each key once and none a key of `funcs`: a later one whose key is
    /// taken (two trait impls of one type with a method of one name, `fmt` of `Display` and of
    /// `Debug`) is named with `_2`, `_3`, … after its name.
    fn opaque_named_apart(
        &mut self,
        funcs: &[codespec_model::Func],
    ) -> Vec<codespec_model::Opaque> {
        let mut taken: BTreeSet<String> = funcs.iter().map(|f| f.key()).collect();
        let mut out = Vec::new();
        for mut o in std::mem::take(&mut self.opaque) {
            let base = o.name.clone();
            let mut k = 2;
            while taken.contains(&o.key()) {
                o.name = format!("{base}_{k}");
                k += 1;
            }
            taken.insert(o.key());
            out.push(o);
        }
        out
    }

    /// The sentinel errors of `enum Error`, their messages read off its `Display` impl.
    fn error_enum_decl(
        &self,
        e: &syn::ItemEnum,
        f: &syn::File,
        refusals: &mut Vec<Refusal>,
    ) -> Option<Vec<ErrorVar>> {
        let (_, vs) = self.error_enum.as_ref()?;
        let pos = self.pos(e.span());
        let at = self.at.borrow().clone();
        let display = f.items.iter().find_map(|i| match i {
            syn::Item::Impl(im)
                if im.trait_.as_ref().is_some_and(|(p, _)| {
                    self.enter(i);
                    self.std_path(p) == Some("std::fmt::Display")
                }) && plain_ident(&im.self_ty).as_deref() == Some("Error") =>
            {
                Some(im)
            }
            _ => None,
        });
        *self.at.borrow_mut() = at;
        let Some(display) = display else {
            refusals.push(Refusal {
                pos,
                what: "enum Error".into(),
                reason: "enum Error without a Display impl: its messages are unknown".into(),
            });
            return None;
        };
        match body::error_messages(self, display, vs) {
            Ok(msgs) => Some(
                e.variants
                    .iter()
                    .filter_map(|v| match vs.get(&v.ident.to_string()) {
                        Some(ErrVariant::Sentinel { go, .. }) => Some(ErrorVar {
                            name: go.clone(),
                            message: msgs.get(&v.ident.to_string()).cloned().unwrap_or_default(),
                            pos: self.pos(v.span()),
                        }),
                        _ => None,
                    })
                    .collect(),
            ),
            Err(reason) => {
                refusals.push(Refusal {
                    pos,
                    what: "enum Error".into(),
                    reason,
                });
                None
            }
        }
    }
}

/// Whether an inherent `impl t` of the file declares a method named `m`.
fn inherent_method(f: &syn::File, t: &str, m: &syn::Ident) -> bool {
    f.items.iter().any(|i| match i {
        syn::Item::Impl(im)
            if im.trait_.is_none() && plain_ident(&im.self_ty).as_deref() == Some(t) =>
        {
            im.items
                .iter()
                .any(|ii| matches!(ii, syn::ImplItem::Fn(func) if func.sig.ident == *m))
        }
        _ => false,
    })
}

/// `MAX_ITEMS` → `max_items`, so a constant's words convert like any other name.
fn screaming(s: &str) -> String {
    if s.chars().any(|c| c.is_lowercase()) {
        s.to_string()
    } else {
        s.to_lowercase()
    }
}

/// Whether `name` is `_<digits>`, the name `cir/1` gives a tuple variant's positional field.
fn tuple_field_name(name: &str) -> bool {
    name.strip_prefix('_')
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// A unit enum's discriminants: explicit integer literals, or one more than the previous, from 0.
fn discriminants(e: &syn::ItemEnum) -> Result<Vec<Expr>, String> {
    let mut next: i128 = 0;
    let mut out = Vec::new();
    for v in &e.variants {
        if let Some((_, d)) = &v.discriminant {
            next = match d {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) => n.base10_parse().map_err(|e| e.to_string())?,
                syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match &*u.expr {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(n),
                        ..
                    }) => -n.base10_parse::<i128>().map_err(|e| e.to_string())?,
                    _ => return Err("a discriminant that is not an integer literal".into()),
                },
                _ => return Err("a discriminant that is not an integer literal".into()),
            };
        }
        out.push(Expr::Lit {
            value: next.to_string(),
            t: int(IntWidth::Platform, true),
        });
        next += 1;
    }
    Ok(out)
}

fn value_from_impls(f: &syn::File, name: &str, variants: &[String]) -> Result<Vec<Expr>, String> {
    for item in &f.items {
        if let syn::Item::Impl(im) = item
            && im.trait_.is_none()
            && plain_ident(&im.self_ty).as_deref() == Some(name)
        {
            for ii in &im.items {
                if let syn::ImplItem::Fn(func) = ii
                    && let Some((_, values)) = value_idiom(func, name, variants)
                {
                    return Ok(values);
                }
            }
        }
    }
    Err("the value() method of the enum is gone".into())
}

pub(crate) fn src_of(n: &impl Spanned) -> String {
    n.span().source_text().unwrap_or_default()
}

/// `s` as text `cir/1` admits outside `doc`, `src`, `value` and `message`: a line break or tab as a
/// space, any other control or format character written as its escape.
pub(crate) fn one_line(s: &str) -> String {
    let flat: String = s
        .chars()
        .map(|c| {
            if matches!(c, '\n' | '\r' | '\t') {
                ' '
            } else {
                c
            }
        })
        .collect();
    codespec_model::printable(&flat).into_owned()
}

pub(crate) fn first_line(s: &str) -> String {
    match s.split_once('\n') {
        Some((l, _)) => format!("{l} …"),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn go_quote_escapes_as_go_does() {
        assert_eq!(super::go_quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
        assert_eq!(super::go_quote("zoë Ω"), "\"zoë Ω\"");
        assert_eq!(super::go_quote("\u{1}"), "\"\\x01\"");
    }
}

---
format: aep.planning-md/3
id: story:rust-reader-no-silent-misread
kind: story
status: implemented
title: The Rust reader misreads nothing silently
summary: serde wire attributes refuse their type; every type, call, trait and macro of the reader's tables is matched by its full path after use, never its last segment.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T02:32:38Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-24T02:32:39Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-24T02:32:39Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Rust reader misreads nothing silently

## Outcome

`codespec-read-rust` refuses what it cannot express exactly instead of reading it as something
else. Two constructs were read wrongly without a refusal:

- serde attributes that change a type's JSON wire form (`#[serde(rename = "…")]`,
  `rename_all`, `skip`, `skip_serializing_if`, `flatten`, `default`, `tag`, `untagged`, `with`,
  `transparent`) were dropped, so the wire names were lost. `cir/1` carries no wire form; the
  draft story `wire-names-from-serde` may add it later.
- a type or call path was matched by its last segment: `tokio::sync::broadcast::Sender<String>`
  read as `chan<- string`, and `other::Vec`, `Rc<other::RefCell<T>>`, `impl other::Display`,
  `#[derive(other::Default)]` or a name a `use` takes from outside the crate as the std or crate
  item of that name.

## Done When

- A struct or enum that carries such a serde attribute on itself, a variant or a field is refused,
  the reason naming the attribute's source; a use of it is refused in turn.
- Every entry of the reader's type table (`String`, `Vec`, `HashMap`, `BTreeMap`, `Rc`,
  `RefCell`, `Rc<dyn Any>`, `Rc<(SyncSender, Receiver)>`, `Box<dyn Tr>`, `Sender`, result
  `Option` and `Result`, primitives, the crate's own types and traits), of its call table
  (`String::from`, `String::new`, `Vec/HashMap/BTreeMap::new`, `Rc::new`, `RefCell::new`,
  `Box::new`, `Rc::clone`, `T::default`, the crate's functions), its trait table (`Display`,
  `std::error::Error`, `derive(Default)`), its map-order check and its std macros is matched by
  the full path the path names after the module's `use` declarations; anything else is refused.
- Regression tests in `adapters/rust-read/tests/reader.rs`, each red before the fix:
  `a_serde_attribute_that_sets_the_wire_form_refuses_the_type_that_carries_it_naming_it`,
  `each_serde_attribute_that_changes_the_wire_form_is_refused_naming_it`,
  `a_type_path_maps_by_its_full_path_so_a_broadcast_sender_is_no_channel`,
  `every_type_table_entry_is_matched_by_its_full_path_not_its_last_segment`,
  `a_use_from_outside_std_shadows_a_name_of_the_type_table`,
  `every_call_table_entry_is_matched_by_its_full_path_not_its_last_segment`,
  `a_name_imported_from_outside_the_crate_is_not_the_crates_item_of_that_name`,
  `display_and_default_are_the_std_ones_only_by_their_full_path`,
  `a_map_renamed_on_import_keeps_its_order`.
- `cargo test -p codespec-read-rust --locked` passes; the checked-in reader outputs are unchanged
  or regenerated through their tasks.

## Scope

- Own: `adapters/rust-read/src/{lib.rs,body.rs}` and `adapters/rust-read/tests/reader.rs`.

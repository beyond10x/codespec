package codespec_read_odin

import "core:testing"
import vmem "core:mem/virtual"

@(test)
typed_calls_and_signatures :: proc(t: ^testing.T) {
	arena: vmem.Arena
	assert(vmem.arena_init_growing(&arena) == nil)
	defer vmem.arena_destroy(&arena)
	context.allocator = vmem.arena_allocator(&arena)
	r: Reader
	collect_source(&r, "a.odin", "package sample\n@(private)\nkeep :: proc(n: int) -> int { return n }\n")
	collect_source(&r, "b.odin", "package sample\nchoose_one :: proc(n: int) -> int {\n x := keep(n)\n if x < int(0) { x = int(1) }\n return x\n}\n")
	doc := module_document(&r)
	testing.expect_value(t, len(r.refusals), 0)
	functions := doc["funcs"].(Array)
	testing.expect_value(t, len(functions), 2)
	f := functions[1].(Object)
	testing.expect_value(t, f["name"].(string), "ChooseOne")
	body := f["body"].(Array)
	testing.expect_value(t, body[0].(Object)["k"].(string), "let")
	testing.expect_value(t, body[1].(Object)["k"].(string), "if")
	testing.expect_value(t, body[2].(Object)["k"].(string), "return")
	sigs := signature_table(&r)["functions"].(Array)
	testing.expect_value(t, len(sigs), 1)
	testing.expect_value(t, sigs[0].(Object)["source"].(string), "choose_one")
}

@(test)
refused_body_keeps_original_source_and_position :: proc(t: ^testing.T) {
	arena: vmem.Arena
	assert(vmem.arena_init_growing(&arena) == nil)
	defer vmem.arena_destroy(&arena)
	context.allocator = vmem.arena_allocator(&arena)
	r: Reader
	collect_source(&r, "f.odin", "package sample\nF :: proc(n: int) -> int {\n defer work(  n )\n return n\n}\n@(private)\nwork :: proc(n: int) {}\n")
	doc := module_document(&r)
	b := doc["funcs"].(Array)[0].(Object)["body"].(Array)
	testing.expect_value(t, b[0].(Object)["k"].(string), "foreign")
	testing.expect_value(t, b[0].(Object)["src"].(string), "defer work(  n )")
	testing.expect_value(t, r.refusals[0].(Object)["pos"].(string), "f.odin:3")
	testing.expect_value(t, b[1].(Object)["k"].(string), "return")
}

@(test)
division_requires_a_live_exclusion_of_the_overflow_pair :: proc(t: ^testing.T) {
	arena: vmem.Arena
	assert(vmem.arena_init_growing(&arena) == nil)
	defer vmem.arena_destroy(&arena)
	context.allocator = vmem.arena_allocator(&arena)
	r: Reader
	collect_source(&r, "div.odin", "package sample\nSafe :: proc(a, b: int) -> int {\n if b == -1 { return -a }\n return a / b\n}\nUnsafe :: proc(a, b: int) -> int {\n d := b\n if d == -1 { return -a }\n d = -1\n return a / d\n}\n")
	doc := module_document(&r)
	testing.expect_value(t, len(r.refusals), 1)
	functions := doc["funcs"].(Array)
	testing.expect_value(t, functions[0].(Object)["body"].(Array)[1].(Object)["k"].(string), "return")
	testing.expect_value(t, r.refusals[0].(Object)["what"].(string), "return a / d")
}

@(test)
integer_literal_domain_has_both_endpoints :: proc(t: ^testing.T) {
	maximum, fits_max := integer_value("9223372036854775807", false)
	minimum, fits_min := integer_value("9_223_372_036_854_775_808", true)
	_, over := integer_value("0x10000000000000000", false)
	_, under := integer_value("9223372036854775809", true)
	testing.expect_value(t, maximum, max(i64))
	testing.expect_value(t, minimum, min(i64))
	testing.expect(t, fits_max && fits_min && !over && !under)
}

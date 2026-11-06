package codespec.read;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

/**
 * The reader's tests, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.read.ReaderTest &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt;</pre>
 *
 * It prints one line per case and a summary {@code N passed, M failed}, and exits 1 when any case
 * fails.
 */
public final class ReaderTest {
    private final Path root;
    private final Path codespec;
    private final Path work;
    private final Path testdata;

    private ReaderTest(Path root, Path codespec, Path work) {
        this.root = root;
        this.codespec = codespec;
        this.work = work;
        this.testdata = root.resolve("adapters/java/testdata");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 3) {
            System.err.println("usage: ReaderTest <repo root> <codespec binary> <work dir>");
            System.exit(2);
        }
        ReaderTest t = new ReaderTest(Path.of(args[0]), Path.of(args[1]), Path.of(args[2]));
        Files.createDirectories(t.work);
        List<String> names = new ArrayList<>();
        List<Case> cases = new ArrayList<>();
        names.add("the_ledger_reads_with_no_refusal");
        cases.add(t::theLedgerReadsWithNoRefusal);
        names.add("the_ledger_read_twice_gives_the_same_bytes");
        cases.add(t::theLedgerReadTwiceGivesTheSameBytes);
        names.add("a_long_balance_is_int64_and_an_int_counter_int32");
        cases.add(t::aLongBalanceIsInt64AndAnIntCounterInt32);
        names.add("the_ledger_holds_every_construct_of_the_story");
        cases.add(t::theLedgerHoldsEveryConstructOfTheStory);
        names.add("after_the_passes_only_the_hash_map_loop_is_unordered_and_the_optional_method_is_comma_ok");
        cases.add(t::afterThePassesOnlyTheHashMapLoopIsUnorderedAndTheOptionalMethodIsCommaOk);
        names.add("the_ledger_document_is_canonical_cir_1");
        cases.add(t::theLedgerDocumentIsCanonicalCir1);
        names.add("a_list_size_is_a_platform_len_converted_to_int32");
        cases.add(t::aListSizeIsAPlatformLenConvertedToInt32);
        names.add("the_reader_version_is_the_workspace_version");
        cases.add(t::theReaderVersionIsTheWorkspaceVersion);
        names.add("integer_max_value_plus_one_is_int32");
        cases.add(t::integerMaxValuePlusOneIsInt32);
        names.add("a_lambda_is_foreign_with_a_refusal_at_its_line");
        cases.add(() -> t.foreignAtMarker("refuse/lambda"));
        names.add("a_stream_chain_is_foreign_with_a_refusal_at_its_line");
        cases.add(() -> t.foreignAtMarker("refuse/stream"));
        names.add("reflection_is_foreign_with_a_refusal_at_its_line");
        cases.add(() -> t.foreignAtMarker("refuse/reflection"));
        names.add("a_runtime_annotation_read_by_the_code_is_foreign_with_a_refusal_at_its_line");
        cases.add(t::aRuntimeAnnotationReadByTheCodeIsForeign);
        names.add("a_synchronized_block_is_foreign_with_a_refusal_at_its_line");
        cases.add(() -> t.foreignAtMarker("refuse/synchronized"));
        names.add("a_generic_method_is_refused_at_its_line");
        cases.add(t::aGenericMethodIsRefusedAtItsLine);
        names.add("a_second_implementation_of_an_interface_is_refused_at_its_line");
        cases.add(t::aSecondImplementationIsRefusedAtItsLine);
        names.add("a_class_extending_a_non_exception_class_is_refused_at_its_line");
        cases.add(t::aClassExtendingANonExceptionClassIsRefusedAtItsLine);
        names.add("adversary_a_reference_field_left_at_its_null_default_is_not_read_as_a_zero_value");
        cases.add(t::adversary_a_reference_field_left_at_its_null_default_is_not_read_as_a_zero_value);
        names.add("adversary_an_add_to_a_list_parameter_is_refused");
        cases.add(t::adversary_an_add_to_a_list_parameter_is_refused);
        names.add("adversary_an_add_through_a_local_alias_of_a_list_is_refused");
        cases.add(t::adversary_an_add_through_a_local_alias_of_a_list_is_refused);
        names.add("adversary_list_equality_by_reference_is_refused");
        cases.add(t::adversary_list_equality_by_reference_is_refused);
        names.add("adversary_a_tree_map_over_enum_keys_is_refused");
        cases.add(t::adversary_a_tree_map_over_enum_keys_is_refused);
        names.add("adversary_a_tree_map_built_by_the_caller_is_refused");
        cases.add(t::adversary_a_tree_map_built_by_the_caller_is_refused);
        names.add("adversary_string_format_verbs_java_and_go_print_differently_are_refused");
        cases.add(t::adversary_string_format_verbs_java_and_go_print_differently_are_refused);
        names.add("adversary_a_static_call_through_an_expression_keeps_the_expression");
        cases.add(t::adversary_a_static_call_through_an_expression_keeps_the_expression);
        names.add("adversary2_a_field_given_a_list_another_name_holds_is_refused");
        cases.add(t::adversary2_a_field_given_a_list_another_name_holds_is_refused);
        names.add("adversary2_a_field_list_handed_out_by_a_getter_is_refused");
        cases.add(t::adversary2_a_field_list_handed_out_by_a_getter_is_refused);
        names.add("adversary2_a_collection_changed_inside_a_for_each_over_itself_is_refused");
        cases.add(t::adversary2_a_collection_changed_inside_a_for_each_over_itself_is_refused);
        names.add("adversary2_a_string_or_boxed_parameter_printed_where_java_prints_null_is_refused");
        cases.add(t::adversary2_a_string_or_boxed_parameter_printed_where_java_prints_null_is_refused);
        int passed = 0;
        int failed = 0;
        for (int i = 0; i < cases.size(); i++) {
            try {
                cases.get(i).run();
                System.out.println("ok   " + names.get(i));
                passed++;
            } catch (Throwable e) {
                System.out.println("FAIL " + names.get(i) + ": " + e);
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed");
        System.exit(failed == 0 ? 0 : 1);
    }

    // ---- the ledger ----

    private Map<String, Object> ledger() throws Exception {
        return Reader.read(testdata.resolve("ledger"));
    }

    private void theLedgerReadsWithNoRefusal() throws Exception {
        Map<String, Object> doc = ledger();
        check(!doc.containsKey("refusals"), "refusals: " + String.valueOf(doc.get("refusals")));
        check("cir/1".equals(doc.get("format")), "format");
        check("ledger".equals(doc.get("name")), "name " + doc.get("name"));
        Map<String, Object> reader = map(doc.get("reader"));
        check("java".equals(reader.get("language")), "reader.language");
        check("codespec-read-java".equals(reader.get("name")), "reader.name");
    }

    private void theLedgerReadTwiceGivesTheSameBytes() throws Exception {
        String a = Json.write(ledger());
        String b = Json.write(ledger());
        check(a.equals(b), "two reads differ");
    }

    private void aLongBalanceIsInt64AndAnIntCounterInt32() throws Exception {
        Map<String, Object> account = byName(list(ledger().get("types")), "Account");
        check("struct".equals(account.get("k")), "Account is " + account.get("k"));
        String balance = Json.write(map(byName(list(account.get("fields")), "balance").get("t")));
        String postings = Json.write(map(byName(list(account.get("fields")), "postings").get("t")));
        check(balance.equals(Json.write(Json.parse("{\"k\":\"int\",\"width\":\"64\",\"signed\":true}"))), "balance " + balance);
        check(postings.equals(Json.write(Json.parse("{\"k\":\"int\",\"width\":\"32\",\"signed\":true}"))), "postings " + postings);
    }

    private void theLedgerHoldsEveryConstructOfTheStory() throws Exception {
        Map<String, Object> doc = ledger();
        List<String> errors = new ArrayList<>();
        for (Object e : list(doc.get("errors"))) {
            errors.add((String) map(e).get("name"));
        }
        check(errors.equals(List.of("InsufficientFunds", "UnknownAccount")), "errors " + errors);
        Map<String, Object> kind = byName(list(doc.get("types")), "Kind");
        check("enum".equals(kind.get("k")) && list(kind.get("variants")).size() == 3, "Kind " + Json.write(kind));
        String funcs = Json.write(doc.get("funcs"));
        for (String needle : new String[] {
            "\"k\": \"switch\"", "\"k\": \"for\"", "\"k\": \"break\"", "\"k\": \"for_range\"",
            "\"name\": \"sprintf\"", "\"name\": \"append\"", "\"name\": \"sorted_keys\"", "\"k\": \"errvar\"",
        }) {
            check(funcs.contains(needle), "no " + needle + " in funcs");
        }
        Map<String, Object> post = funcByKey(doc, "Account.post");
        check(Json.write(post.get("results")).equals(Json.write(Json.parse("[{\"k\":\"error\"}]"))), "Account.post results");
        Map<String, Object> find = funcByKey(doc, "Ledger.find");
        check(Json.write(find.get("results")).equals(Json.write(Json.parse(
            "[{\"k\":\"ptr\",\"elem\":{\"k\":\"named\",\"name\":\"Account\"}},{\"k\":\"bool\"}]"))), "Ledger.find results");
    }

    private void afterThePassesOnlyTheHashMapLoopIsUnorderedAndTheOptionalMethodIsCommaOk() throws Exception {
        Path read = work.resolve("ledger.read.json");
        Path passed = work.resolve("ledger.pass.json");
        Files.writeString(read, Json.write(ledger()), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "pass", read.toString(), "--out", passed.toString())
            .inheritIO().start();
        check(p.waitFor() == 0, "codespec pass failed");
        Map<String, Object> facts = map(map(Json.parse(Files.readString(passed))).get("facts"));
        List<Object> unordered = list(facts.get("unordered_iteration"));
        String hashLoop = "Ledger.java:" + lineOf(testdata.resolve("ledger/Ledger.java"), "for (Kind k : tally.keySet())");
        String treeLoop = "Ledger.java:" + lineOf(testdata.resolve("ledger/Ledger.java"), "for (String id : accounts.keySet())");
        check(unordered.equals(List.of(hashLoop)), "unordered_iteration " + unordered + ", want [" + hashLoop + "], not " + treeLoop);
        check(list(facts.get("comma_ok")).contains("Ledger.find"), "comma_ok " + facts.get("comma_ok"));
    }

    /** The adapter's own bytes, before the CLI re-encodes them, are canonical cir/1. */
    private void theLedgerDocumentIsCanonicalCir1() throws Exception {
        Path doc = work.resolve("ledger.raw.json");
        Files.writeString(doc, Json.write(ledger()), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "validate", "--canonical", doc.toString())
            .inheritIO().start();
        check(p.waitFor() == 0, "codespec validate --canonical failed");
    }

    /**
     * {@code amounts.size()} is {@code len}, of the platform int as the Go reader types it, converted
     * to int32, Java's type of {@code size()}.
     */
    private void aListSizeIsAPlatformLenConvertedToInt32() throws Exception {
        String reach = Json.write(funcByKey(ledger(), "Account.reach").get("body"));
        String want = Json.write(Json.parse("{\"k\":\"conv\",\"name\":\"int32\",\"x\":{\"k\":\"intrinsic\",\"name\":\"len\","
            + "\"args\":[{\"k\":\"field\",\"x\":{\"k\":\"var\",\"name\":\"this\",\"t\":{\"k\":\"ptr\",\"elem\":{\"k\":\"named\","
            + "\"name\":\"Account\"}}},\"name\":\"amounts\",\"t\":{\"k\":\"slice\",\"elem\":{\"k\":\"int\",\"width\":\"64\","
            + "\"signed\":true}}}],\"t\":{\"k\":\"int\",\"width\":\"platform\",\"signed\":true}},"
            + "\"t\":{\"k\":\"int\",\"width\":\"32\",\"signed\":true}}"));
        check(reach.replaceAll("\\s+", "").contains(want.replaceAll("\\s+", "")),
            "no conv(int32, len(this.amounts)) in Account.reach: " + reach.replaceAll("\\s+", " "));
    }

    private void theReaderVersionIsTheWorkspaceVersion() throws Exception {
        String version = null;
        boolean inPackage = false;
        for (String line : Files.readAllLines(root.resolve("Cargo.toml"))) {
            if (line.startsWith("[")) {
                inPackage = line.equals("[workspace.package]");
            } else if (inPackage && line.startsWith("version = ")) {
                version = line.substring("version = ".length()).replace("\"", "").trim();
            }
        }
        check(Main.VERSION.equals(version), "Main.VERSION " + Main.VERSION + ", Cargo.toml " + version);
        check(Main.VERSION.equals(map(ledger().get("reader")).get("version")), "reader.version");
    }

    // ---- types ----

    private void integerMaxValuePlusOneIsInt32() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("overflow"));
        check(!doc.containsKey("refusals"), "refusals: " + String.valueOf(doc.get("refusals")));
        String int32 = "{\"k\":\"int\",\"width\":\"32\",\"signed\":true}";
        String int64 = "{\"k\":\"int\",\"width\":\"64\",\"signed\":true}";
        Object wrap = returned(funcByKey(doc, "wrap"));
        check(same(wrap, "{\"k\":\"binop\",\"op\":\"+\",\"x\":{\"k\":\"lit\",\"value\":\"2147483647\",\"t\":" + int32
            + "},\"y\":{\"k\":\"lit\",\"value\":\"1\",\"t\":" + int32 + "},\"t\":" + int32 + "}"), "wrap returns " + Json.write(wrap));
        Object wide = returned(funcByKey(doc, "wide"));
        check(same(wide, "{\"k\":\"binop\",\"op\":\"+\",\"x\":{\"k\":\"lit\",\"value\":\"9223372036854775807\",\"t\":" + int64
            + "},\"y\":{\"k\":\"lit\",\"value\":\"1\",\"t\":" + int64 + "},\"t\":" + int64 + "}"), "wide returns " + Json.write(wide));
    }

    // ---- refusals ----

    /** The fixture's line marked {@code // refused} is a refusal and a foreign statement. */
    private void foreignAtMarker(String fixture) throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve(fixture));
        String pos = marker(testdata.resolve(fixture));
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + ": " + String.valueOf(doc.get("refusals")));
        check(foreignPositions(doc.get("funcs")).contains(pos), "no foreign statement at " + pos);
    }

    private void aRuntimeAnnotationReadByTheCodeIsForeign() throws Exception {
        foreignAtMarker("refuse/annotation");
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/annotation"));
        String decl = "Audited.java:" + lineOf(testdata.resolve("refuse/annotation/Audited.java"), "public @interface Audited");
        check(refusalPositions(doc).contains(decl), "no refusal of the annotation type at " + decl);
    }

    private void aGenericMethodIsRefusedAtItsLine() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/generic-method"));
        String pos = marker(testdata.resolve("refuse/generic-method"));
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + ": " + String.valueOf(doc.get("refusals")));
        check(funcByKeyOrNull(doc, "first") == null, "the generic method was read");
        check(funcByKeyOrNull(doc, "one") != null, "the method beside it was not read");
    }

    private void aSecondImplementationIsRefusedAtItsLine() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/second-impl"));
        String pos = marker(testdata.resolve("refuse/second-impl"));
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + ": " + String.valueOf(doc.get("refusals")));
        check(byNameOrNull(list(doc.get("types")), "Disc") == null, "the second implementation was read");
        check(byNameOrNull(list(doc.get("types")), "Box") != null, "the first implementation was not read");
    }

    private void aClassExtendingANonExceptionClassIsRefusedAtItsLine() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/extends"));
        String pos = marker(testdata.resolve("refuse/extends"));
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + ": " + String.valueOf(doc.get("refusals")));
        check(byNameOrNull(list(doc.get("types")), "Derived") == null, "the subclass was read");
        check(byNameOrNull(list(doc.get("types")), "Base") != null, "the base class was not read");
    }

    // ---- adversary ----

    /** {@code <file>:<line>} of the first line of {@code adversary/<fixture>/<file>} holding {@code text}. */
    private String adversaryPos(String fixture, String file, String text) throws IOException {
        return file + ":" + lineOf(testdata.resolve("adversary/" + fixture + "/" + file), text);
    }

    private Map<String, Object> adversaryRead(String fixture) throws Exception {
        return Reader.read(testdata.resolve("adversary/" + fixture));
    }

    /**
     * A String, Integer or enum field with no initializer is null in Java until assigned; cir/1's zero
     * value is "", 0 or "". {@code "text=" + text} then prints "text=null" in Java and "text=" read.
     */
    private void adversary_a_reference_field_left_at_its_null_default_is_not_read_as_a_zero_value() throws Exception {
        Map<String, Object> doc = adversaryRead("null-field");
        List<String> at = refusalPositions(doc);
        String use = adversaryPos("null-field", "Label.java", "return \"text=\" + text");
        List<String> missing = new ArrayList<>();
        for (String field : new String[] {"private String text;", "private Integer count;", "private Shade shade;"}) {
            String pos = adversaryPos("null-field", "Label.java", field);
            if (!at.contains(pos) && !at.contains(use)) {
                missing.add(pos);
            }
        }
        check(missing.isEmpty(), "fields read with a zero value where Java holds null, no refusal at " + missing
            + " nor at the use " + use + "; refusals " + at);
    }

    /** {@code out.add(x)} on a parameter: Java's caller sees the add; {@code out = append(out, x)} does not. */
    private void adversary_an_add_to_a_list_parameter_is_refused() throws Exception {
        Map<String, Object> doc = adversaryRead("list-alias");
        String pos = adversaryPos("list-alias", "Fill.java", "out.add(\"x\");");
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + "; refusals " + refusalPositions(doc));
    }

    /** {@code alias = items; alias.add(y)}: Java grows items; the append rebinds only alias. */
    private void adversary_an_add_through_a_local_alias_of_a_list_is_refused() throws Exception {
        Map<String, Object> doc = adversaryRead("list-alias");
        String decl = adversaryPos("list-alias", "Fill.java", "List<String> alias = items;");
        String add = adversaryPos("list-alias", "Fill.java", "alias.add(\"y\");");
        List<String> at = refusalPositions(doc);
        check(at.contains(decl) || at.contains(add), "no refusal at " + decl + " or " + add + "; refusals " + at);
    }

    /** {@code a == b} on two Lists compares identity in Java; on slices it is not that comparison. */
    private void adversary_list_equality_by_reference_is_refused() throws Exception {
        Map<String, Object> doc = adversaryRead("list-alias");
        String pos = adversaryPos("list-alias", "Fill.java", "return a == b;");
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + "; refusals " + refusalPositions(doc));
    }

    /** A TreeMap of enum keys iterates in declaration order; sorted_keys of a string enum sorts the names. */
    private void adversary_a_tree_map_over_enum_keys_is_refused() throws Exception {
        Map<String, Object> doc = adversaryRead("tree-order");
        String pos = adversaryPos("tree-order", "Order.java", "for (Hue h : m.keySet())");
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + "; refusals " + refusalPositions(doc));
    }

    /** A TreeMap the reader did not see built may carry any comparator; sorted_keys assumes the natural one. */
    private void adversary_a_tree_map_built_by_the_caller_is_refused() throws Exception {
        Map<String, Object> doc = adversaryRead("tree-order");
        String pos = adversaryPos("tree-order", "Order.java", "for (String k : m.keySet())");
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + "; refusals " + refusalPositions(doc));
    }

    /** String.format's text goes into sprintf unchanged, though Formatter and Go fmt disagree on it. */
    private void adversary_string_format_verbs_java_and_go_print_differently_are_refused() throws Exception {
        Map<String, Object> doc = adversaryRead("format-verbs");
        List<String> at = refusalPositions(doc);
        List<String> missing = new ArrayList<>();
        for (String line : new String[] {
            "String.format(\"%s\", n)", "String.format(\"%x\", n)", "String.format(\"a%nb\")",
            "String.format(\"%1$s %1$s\", s)", "String.format(\"%s\", l)", "String.format(f, n)",
        }) {
            String pos = adversaryPos("format-verbs", "Fmt.java", line);
            if (!at.contains(pos)) {
                missing.add(pos + " " + line);
            }
        }
        check(missing.isEmpty(), "read without a refusal: " + missing);
    }

    /** {@code bump().zero()}: Java runs bump() and then the static zero(); the read call drops bump(). */
    private void adversary_a_static_call_through_an_expression_keeps_the_expression() throws Exception {
        Map<String, Object> doc = adversaryRead("static-receiver");
        String pos = adversaryPos("static-receiver", "Side.java", "int z = bump().zero();");
        String run = Json.write(funcByKey(doc, "Side.run").get("body"));
        check(refusalPositions(doc).contains(pos) || run.contains("\"name\": \"bump\""),
            "bump() is gone from Side.run and nothing is refused at " + pos + ": " + run.replaceAll("\\s+", " "));
    }

    // ---- adversary pass 2 ----

    private String adversary2Pos(String fixture, String file, String text) throws IOException {
        return file + ":" + lineOf(testdata.resolve("adversary2/" + fixture + "/" + file), text);
    }

    /** Fails listing every group none of whose positions carries a refusal. */
    private void adversary2RefusedAtOneOf(String fixture, String file, String[][] groups) throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("adversary2/" + fixture));
        List<String> at = refusalPositions(doc);
        List<String> missing = new ArrayList<>();
        for (String[] group : groups) {
            List<String> poss = new ArrayList<>();
            boolean hit = false;
            for (String text : group) {
                String pos = adversary2Pos(fixture, file, text);
                poss.add(pos);
                hit |= at.contains(pos);
            }
            if (!hit) {
                missing.add(String.valueOf(poss));
            }
        }
        check(missing.isEmpty(), "read without a refusal at any of " + missing + "; refusals " + at);
    }

    /**
     * {@code this.items = items} (a parameter) or {@code this.held = l} (a local), then an add through
     * the field: Java's other name sees the add, the read append rebinds only the field.
     */
    private void adversary2_a_field_given_a_list_another_name_holds_is_refused() throws Exception {
        adversary2RefusedAtOneOf("shared-field", "Bag.java", new String[][] {
            {"this.items = items;", "this.items.add(s);", "Bag b = new Bag(l);"},
            {"this.held = l;", "this.held.add(\"z\");"},
        });
    }

    /** {@code return this.items} hands the field's list out; a later add through the field is seen there. */
    private void adversary2_a_field_list_handed_out_by_a_getter_is_refused() throws Exception {
        adversary2RefusedAtOneOf("escaping-field", "Box.java", new String[][] {
            {"return this.items;", "this.items.add(s);", "List<String> v = b.view();"},
        });
    }

    /** An add or put inside a for-each over the same collection: Java throws, the range runs on. */
    private void adversary2_a_collection_changed_inside_a_for_each_over_itself_is_refused() throws Exception {
        adversary2RefusedAtOneOf("grow-in-loop", "Grow.java", new String[][] {
            {"for (String s : this.items)", "this.items.add(s);"},
            {"for (String s : l)", "            l.add(s);"},
            {"for (String k : m.keySet())", "m.put(k + \"!\", 2);"},
        });
    }

    /** A String or Integer parameter may be null: Java prints "null", the read prints "" or 0. */
    private void adversary2_a_string_or_boxed_parameter_printed_where_java_prints_null_is_refused() throws Exception {
        adversary2RefusedAtOneOf("null-param", "Greet.java", new String[][] {
            {"return \"hi \" + name;", "public static String hello(String name)"},
            {"return String.format(\"%d items\", n);", "public static String count(Integer n)"},
        });
    }

    // ---- helpers ----

    private static void check(boolean ok, String what) {
        if (!ok) {
            throw new AssertionError(what);
        }
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> map(Object o) {
        check(o instanceof Map, "not an object: " + o);
        return (Map<String, Object>) o;
    }

    @SuppressWarnings("unchecked")
    private static List<Object> list(Object o) {
        if (o == null) {
            return List.of();
        }
        check(o instanceof List, "not an array: " + o);
        return (List<Object>) o;
    }

    private static Map<String, Object> byNameOrNull(List<Object> xs, String name) {
        for (Object x : xs) {
            if (name.equals(map(x).get("name"))) {
                return map(x);
            }
        }
        return null;
    }

    private static Map<String, Object> byName(List<Object> xs, String name) {
        Map<String, Object> m = byNameOrNull(xs, name);
        check(m != null, "no " + name);
        return m;
    }

    private static Map<String, Object> funcByKeyOrNull(Map<String, Object> doc, String key) {
        for (Object f : list(doc.get("funcs"))) {
            Map<String, Object> fn = map(f);
            String k = (String) fn.get("name");
            if (fn.containsKey("recv")) {
                Map<String, Object> t = map(map(fn.get("recv")).get("t"));
                if ("ptr".equals(t.get("k"))) {
                    t = map(t.get("elem"));
                }
                k = t.get("name") + "." + k;
            }
            if (key.equals(k)) {
                return fn;
            }
        }
        return null;
    }

    private static Map<String, Object> funcByKey(Map<String, Object> doc, String key) {
        Map<String, Object> f = funcByKeyOrNull(doc, key);
        check(f != null, "no func " + key);
        return f;
    }

    private static Object returned(Map<String, Object> fn) {
        List<Object> body = list(fn.get("body"));
        Map<String, Object> ret = map(body.get(body.size() - 1));
        check("return".equals(ret.get("k")), "last statement is " + ret.get("k"));
        return list(ret.get("results")).get(0);
    }

    private static boolean same(Object got, String want) {
        return Json.write(got).equals(Json.write(Json.parse(want)));
    }

    private static List<String> refusalPositions(Map<String, Object> doc) {
        List<String> out = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            out.add((String) map(r).get("pos"));
        }
        return out;
    }

    /** Positions of every foreign statement under {@code o}. */
    private static List<String> foreignPositions(Object o) {
        List<String> out = new ArrayList<>();
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if ("foreign".equals(m.get("k")) && "java".equals(m.get("lang"))) {
                out.add((String) m.get("pos"));
            }
            for (Object v : m.values()) {
                out.addAll(foreignPositions(v));
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                out.addAll(foreignPositions(v));
            }
        }
        return out;
    }

    /** {@code <file>:<line>} of the one line marked {@code // refused} in a fixture. */
    private static String marker(Path dir) throws IOException {
        List<String> found = new ArrayList<>();
        try (Stream<Path> files = Files.list(dir)) {
            for (Path f : files.sorted().toList()) {
                List<String> lines = Files.readAllLines(f);
                for (int i = 0; i < lines.size(); i++) {
                    if (lines.get(i).contains("// refused")) {
                        found.add(f.getFileName() + ":" + (i + 1));
                    }
                }
            }
        }
        check(found.size() == 1, "markers in " + dir + ": " + found);
        return found.get(0);
    }

    private static int lineOf(Path file, String text) throws IOException {
        List<String> lines = Files.readAllLines(file);
        for (int i = 0; i < lines.size(); i++) {
            if (lines.get(i).contains(text)) {
                return i + 1;
            }
        }
        throw new AssertionError("no " + text + " in " + file);
    }
}

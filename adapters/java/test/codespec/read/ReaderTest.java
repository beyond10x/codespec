package codespec.read;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.stream.Stream;

/**
 * The reader's tests, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.read.ReaderTest &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt; &lt;adapters dir&gt;</pre>
 *
 * The adapters directory holds {@code codespec-read-java}, {@code codespec-write-go} and {@code
 * codespec-run-go}, as {@code task adapters} builds them. It prints one line per case and a summary
 * {@code N passed, M failed}, and exits 1 when any case fails.
 */
public final class ReaderTest {
    private final Path root;
    private final Path codespec;
    private final Path work;
    private final Path adapters;
    private final Path testdata;

    private ReaderTest(Path root, Path codespec, Path work, Path adapters) {
        this.root = root;
        this.codespec = codespec;
        this.work = work;
        this.adapters = adapters;
        this.testdata = root.resolve("adapters/java/testdata");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 4) {
            System.err.println("usage: ReaderTest <repo root> <codespec binary> <work dir> <adapters dir>");
            System.exit(2);
        }
        ReaderTest t = new ReaderTest(Path.of(args[0]), Path.of(args[1]), Path.of(args[2]), Path.of(args[3]));
        Files.createDirectories(t.work);
        List<String> names = new ArrayList<>();
        List<Case> cases = new ArrayList<>();
        names.add("the_ledger_reads_with_no_refusal");
        cases.add(t::theLedgerReadsWithNoRefusal);
        names.add("the_example_ledger_post_passes_account_posts_error_on_unchanged");
        cases.add(t::theExampleLedgerPostPassesAccountPostsErrorOnUnchanged);
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
        names.add("a_reference_field_some_constructor_leaves_unassigned_reads_as_a_nil_pointer");
        cases.add(t::aReferenceFieldSomeConstructorLeavesUnassignedReadsAsANilPointer);
        names.add("a_use_of_an_unassigned_field_where_java_prints_computes_with_or_hands_on_null_is_refused");
        cases.add(t::aUseOfAnUnassignedFieldWhereJavaHandsOnNullIsRefused);
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
        names.add("an_interface_with_two_implementations_is_a_cir_1_interface");
        cases.add(t::anInterfaceWithTwoImplementationsIsACir1Interface);
        names.add("the_edges_of_the_kit_constructs_are_refused_at_their_lines");
        cases.add(t::theEdgesOfTheKitConstructsAreRefusedAtTheirLines);
        names.add("a_float_literal_is_spelled_as_cir_1_spells_it");
        cases.add(t::aFloatLiteralIsSpelledAsCir1SpellsIt);
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
        names.add("a_generic_type_outside_the_table_is_refused_with_the_type_named");
        cases.add(t::aGenericTypeOutsideTheTableIsRefusedWithTheTypeNamed);
        names.add("adversary3_a_package_under_javax_reads_its_own_classes_as_before");
        cases.add(t::adversary3_a_package_under_javax_reads_its_own_classes_as_before);
        names.add("adversary_null_an_assignment_to_a_null_until_assigned_field_evaluates_its_receiver_before_its_value");
        cases.add(t::adversary_null_an_assignment_to_a_null_until_assigned_field_evaluates_its_receiver_before_its_value);
        names.add("adversary_null_a_read_where_java_goes_on_with_the_null_is_refused");
        cases.add(t::adversary_null_a_read_where_java_goes_on_with_the_null_is_refused);
        names.add("adversary_null_a_call_of_a_refused_second_constructor_is_refused_not_a_crash");
        cases.add(t::adversary_null_a_call_of_a_refused_second_constructor_is_refused_not_a_crash);
        names.add("every_kit_case_with_a_java_source_reads_without_refusal_and_prints_its_expected_txt_through_the_go_writer");
        cases.add(t::everyKitCaseWithAJavaSourcePrintsItsExpectedTxtThroughTheGoWriter);
        names.add("every_claimed_feature_is_the_subject_of_a_kit_case_with_a_java_source");
        cases.add(t::everyClaimedFeatureIsTheSubjectOfAKitCaseWithAJavaSource);
        names.add("a_project_directory_is_read_one_module_per_package");
        cases.add(t::aProjectDirectoryIsReadOneModulePerPackage);
        names.add("a_project_of_several_packages_read_without_naming_one_is_refused_naming_them_and_the_variable");
        cases.add(t::aProjectOfSeveralPackagesReadWithoutNamingOneIsRefusedNamingThemAndTheVariable);
        names.add("a_project_read_without_its_jars_is_refused_naming_the_variable");
        cases.add(t::aProjectReadWithoutItsJarsIsRefusedNamingTheVariable);
        names.add("quarkus_entry_points_are_http_methods_with_joined_paths_and_scheduled_methods");
        cases.add(t::quarkusEntryPointsAreHttpMethodsWithJoinedPathsAndScheduledMethods);
        names.add("quarkus_beans_and_injection_points_resolve_to_the_one_bean_or_to_an_extern");
        cases.add(t::quarkusBeansAndInjectionPointsResolveToTheOneBeanOrToAnExtern);
        names.add("a_call_into_a_jar_is_an_extern_with_its_declared_signature");
        cases.add(t::aCallIntoAJarIsAnExternWithItsDeclaredSignature);
        names.add("quarkus_refuses_only_what_it_cannot_map");
        cases.add(t::quarkusRefusesOnlyWhatItCannotMap);
        names.add("quarkus_read_twice_gives_the_same_bytes");
        cases.add(t::quarkusReadTwiceGivesTheSameBytes);
        names.add("effects_over_the_quarkus_read_give_the_persisting_method_db_and_the_client_method_net");
        cases.add(t::effectsOverTheQuarkusReadGiveThePersistingMethodDbAndTheClientMethodNet);
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

    /**
     * {@code examples/ledger}: {@code Account.post} throws on an overdrawing withdrawal and {@code
     * Ledger.post} calls it without catching, so the call reads as the error-only propagation form
     * {@code err := a.post(kind, amount); if err != nil { return err }}, with no refusal.
     */
    private void theExampleLedgerPostPassesAccountPostsErrorOnUnchanged() throws Exception {
        Map<String, Object> doc = Reader.read(root.resolve("examples/ledger/source"));
        check(!doc.containsKey("refusals"), "refusals: " + String.valueOf(doc.get("refusals")));
        Map<String, Object> account = funcByKey(doc, "Account.post");
        check(Json.write(account.get("results")).equals(Json.write(Json.parse("[{\"k\":\"error\"}]"))),
            "Account.post results " + Json.write(account.get("results")).replaceAll("\\s+", " "));
        check(Json.write(account.get("body")).contains("\"name\": \"InsufficientFunds\""),
            "Account.post does not return InsufficientFunds");
        List<Object> body = list(funcByKey(doc, "Ledger.post").get("body"));
        boolean found = false;
        for (int i = 0; i + 1 < body.size() && !found; i++) {
            Map<String, Object> let = map(body.get(i));
            if (!"let".equals(let.get("k")) || list(let.get("names")).size() != 1) {
                continue;
            }
            Map<String, Object> call = map(list(let.get("rhs")).get(0));
            if (!"method".equals(call.get("k")) || !"post".equals(call.get("name"))) {
                continue;
            }
            String err = (String) list(let.get("names")).get(0);
            Map<String, Object> iff = map(body.get(i + 1));
            check("if".equals(iff.get("k")), "a.post(…) is followed by " + iff.get("k"));
            List<Object> then = list(iff.get("then"));
            Map<String, Object> ret = map(then.get(then.size() - 1));
            List<Object> rs = list(ret.get("results"));
            check("return".equals(ret.get("k")) && same(rs.get(rs.size() - 1),
                "{\"k\":\"var\",\"name\":\"" + err + "\",\"t\":{\"k\":\"error\"}}"),
                "the if after a.post(…) does not return " + err + ": " + Json.write(iff).replaceAll("\\s+", " "));
            found = true;
        }
        check(found, "no error-only let of a.post(…) in Ledger.post: " + Json.write(body).replaceAll("\\s+", " "));
        check(!Json.write(body).contains("\"name\": \"InsufficientFunds\""), "Ledger.post still returns InsufficientFunds");
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
        // A constant expression that wraps is read as the literal of Java's value: Go would add exactly.
        check(same(wrap, "{\"k\":\"lit\",\"value\":\"-2147483648\",\"t\":" + int32 + "}"), "wrap returns " + Json.write(wrap));
        Object wide = returned(funcByKey(doc, "wide"));
        check(same(wide, "{\"k\":\"lit\",\"value\":\"-9223372036854775808\",\"t\":" + int64 + "}"), "wide returns " + Json.write(wide));
    }

    // ---- fields that hold null until assigned ----

    private static final String PTR_STRING = "{\"k\":\"ptr\",\"elem\":{\"k\":\"string\"}}";
    private static final String PTR_INT32 = "{\"k\":\"ptr\",\"elem\":{\"k\":\"int\",\"width\":\"32\",\"signed\":true}}";
    private static final String THIS_NOTE = "{\"k\":\"var\",\"name\":\"this\",\"t\":{\"k\":\"ptr\",\"elem\":{\"k\":\"named\",\"name\":\"Note\"}}}";

    /**
     * {@code testdata/nullable}: a String and an Integer field without an initializer, which no
     * constructor assigns, read as {@code ptr(string)} and {@code ptr(int32)} with no refusal; a Note
     * field stays {@code ptr(Note)}; {@code text != null} a comparison of the field itself with
     * {@code nil}; a read as a receiver a dereference; an assignment a pointer to a fresh local
     * holding the value, {@code null} a {@code nil}, another such field that field's pointer.
     */
    private void aReferenceFieldSomeConstructorLeavesUnassignedReadsAsANilPointer() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("nullable"));
        check(!doc.containsKey("refusals"), "refusals: " + String.valueOf(doc.get("refusals")));
        List<Object> fields = list(byName(list(doc.get("types")), "Note").get("fields"));
        check(same(byName(fields, "text").get("t"), PTR_STRING), "text is " + Json.write(byName(fields, "text").get("t")));
        check(same(byName(fields, "count").get("t"), PTR_INT32), "count is " + Json.write(byName(fields, "count").get("t")));
        check(same(byName(fields, "next").get("t"), "{\"k\":\"ptr\",\"elem\":{\"k\":\"named\",\"name\":\"Note\"}}"),
            "next, a class of the package, is " + Json.write(byName(fields, "next").get("t")));
        String text = "{\"k\":\"field\",\"x\":" + THIS_NOTE + ",\"name\":\"text\",\"t\":" + PTR_STRING + "}";
        String count = "{\"k\":\"field\",\"x\":" + THIS_NOTE + ",\"name\":\"count\",\"t\":" + PTR_INT32 + "}";
        Object hasText = returned(funcByKey(doc, "Note.hasText"));
        check(same(hasText, "{\"k\":\"binop\",\"op\":\"!=\",\"x\":" + text + ",\"y\":{\"k\":\"nil\",\"t\":" + PTR_STRING
            + "},\"t\":{\"k\":\"bool\"}}"), "hasText returns " + Json.write(hasText));
        Object hasCount = returned(funcByKey(doc, "Note.hasCount"));
        check(same(hasCount, "{\"k\":\"binop\",\"op\":\"!=\",\"x\":" + count + ",\"y\":{\"k\":\"nil\",\"t\":" + PTR_INT32
            + "},\"t\":{\"k\":\"bool\"}}"), "hasCount returns " + Json.write(hasCount));
        Object holds = returned(funcByKey(doc, "Note.holds"));
        check(same(holds, "{\"k\":\"binop\",\"op\":\"==\",\"x\":{\"k\":\"unop\",\"op\":\"*\",\"x\":" + text
            + ",\"t\":{\"k\":\"string\"}},\"y\":{\"k\":\"var\",\"name\":\"s\",\"t\":{\"k\":\"string\"}},\"t\":{\"k\":\"bool\"}}"),
            "holds returns " + Json.write(holds));
        List<Object> write = list(funcByKey(doc, "Note.write").get("body"));
        String pos = "Note.java:" + lineOf(testdata.resolve("nullable/Note.java"), "this.text = t;");
        check(same(write, "[{\"k\":\"let\",\"names\":[\"text\"],\"rhs\":[{\"k\":\"var\",\"name\":\"t\",\"t\":{\"k\":\"string\"}}],"
            + "\"pos\":\"" + pos + "\"},{\"k\":\"assign\",\"lhs\":[" + text + "],\"rhs\":[{\"k\":\"unop\",\"op\":\"&\",\"x\":{\"k\":\"var\","
            + "\"name\":\"text\",\"t\":{\"k\":\"string\"}},\"t\":" + PTR_STRING + "}],\"pos\":\"" + pos + "\"}]"),
            "write is " + Json.write(write));
        List<Object> clear = list(funcByKey(doc, "Note.clear").get("body"));
        check(same(map(clear.get(0)).get("rhs"), "[{\"k\":\"nil\",\"t\":" + PTR_STRING + "}]"), "clear is " + Json.write(clear));
        List<Object> copy = list(funcByKey(doc, "copyText").get("body"));
        check(same(map(copy.get(0)).get("rhs"), "[{\"k\":\"field\",\"x\":{\"k\":\"var\",\"name\":\"from\",\"t\":{\"k\":\"ptr\","
            + "\"elem\":{\"k\":\"named\",\"name\":\"Note\"}}},\"name\":\"text\",\"t\":" + PTR_STRING + "}]"), "copyText is " + Json.write(copy));
    }

    /**
     * A field that holds null until assigned, printed by concatenation or {@code String.format}, an
     * operand of an operator, the target of {@code +=} or {@code ++}, an argument of a library method
     * or the receiver of a collection write: refused at the use. Its receiver use {@code
     * text.equals(..)}, which throws in Java where the written program stops, is read.
     */
    private void aUseOfAnUnassignedFieldWhereJavaHandsOnNullIsRefused() throws Exception {
        Path dir = testdata.resolve("refuse/null-use");
        Map<String, Object> doc = Reader.read(dir);
        Path tag = dir.resolve("Tag.java");
        List<String> at = refusalPositions(doc);
        List<String> missing = new ArrayList<>();
        for (String use : new String[] {
            "String.format(\"%s\", text)", "count * 2", "count == 3", "count += 1", "count++", "-count",
            "other.equals(text)", "shade == Shade.DARK", "words.add(w)",
        }) {
            String pos = "Tag.java:" + lineOf(tag, use);
            if (!at.contains(pos)) {
                missing.add(use + " at " + pos);
            }
        }
        check(missing.isEmpty(), "not refused: " + missing + "; refusals " + at);
        for (String field : new String[] {"private String text;", "private Integer count;", "private Shade shade;", "private List<String> words;"}) {
            String pos = "Tag.java:" + lineOf(tag, field);
            check(!at.contains(pos), "the field itself refused at " + pos);
        }
        String hello = "Tag.java:" + lineOf(tag, "text.equals(\"hello\")");
        check(!at.contains(hello), "the receiver use refused at " + hello);
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

    /**
     * An interface of the package with two implementations is a cir/1 interface: both classes are
     * read, the interface lists its method, and nothing is refused.
     */
    private void anInterfaceWithTwoImplementationsIsACir1Interface() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("interface/two-impls"));
        check(!doc.containsKey("refusals"), "refusals: " + String.valueOf(doc.get("refusals")));
        check(byNameOrNull(list(doc.get("types")), "Disc") != null, "the second implementation was not read");
        check(byNameOrNull(list(doc.get("types")), "Box") != null, "the first implementation was not read");
        Map<String, Object> shape = byName(list(doc.get("interfaces")), "Shape");
        check(same(shape.get("methods"), "[{\"name\":\"Area\",\"params\":[],\"results\":[{\"k\":\"int\",\"width\":\"64\",\"signed\":true}]}]"),
            "Shape's methods " + Json.write(shape.get("methods")));
    }

    /**
     * Each line marked {@code // refused} in {@code refuse/kit-limits} is a refusal: the edges of
     * what the conformance kit work maps (shifts, labels, floats, arrays, generics, lambdas, static
     * fields, formatted exceptions) where Java and cir/1 would differ.
     */
    private void theEdgesOfTheKitConstructsAreRefusedAtTheirLines() throws Exception {
        Path dir = testdata.resolve("refuse/kit-limits");
        Map<String, Object> doc = Reader.read(dir);
        List<String> want = new ArrayList<>();
        try (Stream<Path> files = Files.list(dir)) {
            for (Path f : files.sorted().toList()) {
                List<String> lines = Files.readAllLines(f);
                for (int i = 0; i < lines.size(); i++) {
                    if (lines.get(i).contains("// refused")) {
                        want.add(f.getFileName() + ":" + (i + 1));
                    }
                }
            }
        }
        check(want.size() >= 10, "markers in " + dir + ": " + want);
        List<String> missing = new ArrayList<>(want);
        missing.removeAll(refusalPositions(doc));
        check(missing.isEmpty(), "no refusal at " + missing + ": " + String.valueOf(doc.get("refusals")));
        List<String> extra = new ArrayList<>(refusalPositions(doc));
        extra.removeAll(want);
        check(extra.isEmpty(), "refusals at unmarked lines " + extra + ": " + String.valueOf(doc.get("refusals")));
        check(funcByKeyOrNull(doc, "readable") != null, "the method beside them was not read");
    }

    /** cir/1's spelling of float literals, which strict reading requires ({@code float_spelling} of crates/model). */
    private void aFloatLiteralIsSpelledAsCir1SpellsIt() throws Exception {
        String[][] want = {
            {"0.25", "0.25"}, {"1.5e300", "1.5e300"}, {"4", "4"}, {"1000", "1000"}, {"1e21", "1e21"},
            {"1e20", "100000000000000000000"}, {"1.5e-7", "1.5e-7"}, {"0.000001", "0.000001"}, {"-0.0", "-0"},
            {"0.1", "0.1"}, {"5e-324", "5e-324"}, {"1.7976931348623157e308", "1.7976931348623157e308"},
        };
        for (String[] w : want) {
            String got = Reader.floatSpelling(Double.parseDouble(w[0]), false);
            check(got.equals(w[1]), w[0] + " spelled " + got + ", want " + w[1]);
        }
        check(Reader.floatSpelling(0.1f, true).equals("0.1"), "0.1f spelled " + Reader.floatSpelling(0.1f, true));
        check(Reader.floatSpelling(1e21f, true).equals("1e21"), "1e21f spelled " + Reader.floatSpelling(1e21f, true));
        check(Reader.floatSpelling(16777217f, true).equals("16777216"), "16777217f spelled " + Reader.floatSpelling(16777217f, true));
    }

    private void aClassExtendingANonExceptionClassIsRefusedAtItsLine() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/extends"));
        String pos = marker(testdata.resolve("refuse/extends"));
        check(refusalPositions(doc).contains(pos), "no refusal at " + pos + ": " + String.valueOf(doc.get("refusals")));
        check(byNameOrNull(list(doc.get("types")), "Derived") == null, "the subclass was read");
        check(byNameOrNull(list(doc.get("types")), "Base") != null, "the base class was not read");
    }

    /** The reason for {@code Map<String, List<Double>>}: the innermost type the table lacks. */
    private static final String MAP_OF_DOUBLE_LISTS_REASON = "type java.lang.Double is outside the reader's types";
    /** The reason for {@code Set<String>}: the type as {@code TypeMirror.toString()} prints it. */
    private static final String SET_OF_STRINGS_REASON = "type java.util.Set<java.lang.String> is outside the reader's types";

    /**
     * A parameter type outside the JVM type table (codespec.jvm.JvmTypes) refuses its method with the
     * reason the reader has always given, byte for byte.
     */
    private void aGenericTypeOutsideTheTableIsRefusedWithTheTypeNamed() throws Exception {
        Path dir = testdata.resolve("refuse/generic-type");
        Map<String, Object> doc = Reader.read(dir);
        List<String> got = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> m = map(r);
            got.add(m.get("pos") + " " + m.get("what") + ": " + m.get("reason"));
        }
        Path shelf = dir.resolve("Shelf.java");
        List<String> want = List.of(
            "Shelf.java:" + lineOf(shelf, "ratings(Map<String, List<Double>>") + " Shelf.ratings(…): " + MAP_OF_DOUBLE_LISTS_REASON,
            "Shelf.java:" + lineOf(shelf, "tags(Set<String>") + " Shelf.tags(…): " + SET_OF_STRINGS_REASON);
        check(got.equals(want), "refusals " + got + ", want " + want);
        check(funcByKeyOrNull(doc, "count") != null, "the method beside them was not read");
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

    // ---- adversary pass 3 ----

    /**
     * Byte-equality of the reader's output: a package named {@code javax.lending} is the package's
     * own code, and its class, enum, one-implementation interface and exception hierarchy map as they
     * do in any other package. The reader on the base read it with no refusal and {@code books} as a
     * slice of {@code *Book}.
     */
    private void adversary3_a_package_under_javax_reads_its_own_classes_as_before() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("javax-package"));
        check(!doc.containsKey("refusals"), "refusals: " + String.valueOf(doc.get("refusals")));
        Map<String, Object> library = byNameOrNull(list(doc.get("types")), "Library");
        check(library != null, "Library was not read");
        String books = null;
        for (Object f : list(library.get("fields"))) {
            if ("books".equals(map(f).get("name"))) {
                books = Json.write(map(f).get("t")).replaceAll("\\s+", "");
            }
        }
        check("{\"k\":\"slice\",\"elem\":{\"k\":\"ptr\",\"elem\":{\"k\":\"named\",\"name\":\"Book\"}}}".equals(books),
            "Library.books is " + books);
    }

    // ---- adversary: fields that hold null until assigned ----

    private String adversary3Pos(String fixture, String file, String text) throws IOException {
        return file + ":" + lineOf(testdata.resolve("adversary3/" + fixture + "/" + file), text);
    }

    /**
     * Evaluation order: {@code s.pick().label = s.stamp()} calls pick before stamp in Java. Read as
     * {@code let label = s.stamp(); s.pick().label = &label}, the written program calls stamp first
     * and returns "value first" where Java returns "receiver first" (Go and Rust both, measured). A
     * field every constructor assigns keeps Java's order in Go. Refusing the line also passes.
     */
    private void adversary_null_an_assignment_to_a_null_until_assigned_field_evaluates_its_receiver_before_its_value() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("adversary3/null-field-order"));
        String pos = adversary3Pos("null-field-order", "Shelf.java", "s.pick().label = s.stamp();");
        if (refusalPositions(doc).contains(pos)) {
            return;
        }
        String body = Json.write(funcByKey(doc, "labelAfterPick").get("body")).replaceAll("\\s+", "");
        int pick = body.indexOf("\"name\":\"pick\"");
        int stamp = body.indexOf("\"name\":\"stamp\"");
        check(pick >= 0 && stamp >= 0, "no call of pick or stamp in " + body);
        check(pick < stamp, "the value s.stamp() is evaluated before the receiver s.pick() at " + pos + ": " + body);
    }

    /**
     * A getter returning the field, a package method's argument and a local's initializer: on a fresh
     * object Java returns null, "shelved" and "kept"; the written Go and Rust dereference the nil
     * pointer and panic (measured), with no refusal. Each such read is refused.
     */
    private void adversary_null_a_read_where_java_goes_on_with_the_null_is_refused() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("adversary3/null-field-escape"));
        List<String> at = refusalPositions(doc);
        List<String> missing = new ArrayList<>();
        for (String use : new String[] {"return title;", "return shelved(new Card().title);", "String t = c.title;"}) {
            String pos = adversary3Pos("null-field-escape", "Card.java", use);
            if (!at.contains(pos)) {
                missing.add(use + " at " + pos);
            }
        }
        check(missing.isEmpty(), "read where Java goes on with null and the written program panics: " + missing + "; refusals " + at);
    }

    /**
     * A field assigned in only one of two constructors: the class now reads (the field is a
     * pointer), the second constructor is refused, and {@code new Card(t)} was read against the first
     * constructor's empty parameter list, an IndexOutOfBoundsException out of the reader.
     */
    private void adversary_null_a_call_of_a_refused_second_constructor_is_refused_not_a_crash() throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("adversary3/two-constructors"));
        String use = adversary3Pos("two-constructors", "Card.java", "return new Card(t).title != null;");
        check(refusalPositions(doc).contains(use), "no refusal at " + use + "; refusals " + refusalPositions(doc));
    }

    // ---- the conformance kit ----

    private Path kit() {
        return root.resolve("conformance/cases");
    }

    /** The kit cases holding {@code java/Case.java}, by name, in name order. */
    private List<String> javaCases() throws IOException {
        List<String> out = new ArrayList<>();
        try (Stream<Path> dirs = Files.list(kit())) {
            for (Path d : dirs.sorted().toList()) {
                if (Files.isDirectory(d.resolve("java"))) {
                    check(Files.isRegularFile(d.resolve("java/Case.java")), d.getFileName() + "/java holds no Case.java");
                    out.add(d.getFileName().toString());
                }
            }
        }
        return out;
    }

    /** The subject of a kit case: the first feature its {@code features.json} lists. */
    private String subject(String kase) throws IOException {
        Map<String, Object> f = map(Json.parse(Files.readString(kit().resolve(kase).resolve("features.json"))));
        return (String) list(f.get("features")).get(0);
    }

    /**
     * Runs a program to completion with its stdout to {@code out}; a non-zero exit fails, naming
     * the step and quoting stderr.
     */
    private static void run(String step, Path out, Path err, String... cmd) throws Exception {
        Process p = new ProcessBuilder(cmd).redirectOutput(out.toFile()).redirectError(err.toFile()).start();
        int code = p.waitFor();
        if (code != 0) {
            throw new AssertionError(step + " exited " + code + ": " + Files.readString(err).strip());
        }
    }

    /**
     * One kit case's {@code java/}, as {@code docs/conformance.md} § 6 checks a reader: read through
     * {@code codespec read --lang java} with no refusal and no foreign statement, its module named as
     * the case's package, passed, written as Go and run by {@code codespec-run-go} on the case's
     * {@code calls.json}, printing its {@code expected.txt}. Its subject is claimed. The module and
     * the subject are left in {@code <work>/kit/<case>/} for {@code task adapters-check}, which
     * checks the module uses the subject ({@code codespec-conformance features}). A case whose
     * subject is one of {@link #REFUSED} is read into {@code <work>/refused/<case>/} instead, and
     * holds the one refusal its entry names, at the line its entry names, and no foreign statement.
     */
    private void kitCase(String kase) throws Exception {
        Path dir = kit().resolve(kase);
        String subject = subject(kase);
        boolean refusedCase = REFUSED.containsKey(subject);
        Path w = work.resolve(refusedCase ? "refused" : "kit").resolve(kase);
        Files.createDirectories(w);
        Path read = w.resolve("read.cir.json");
        Path passed = w.resolve("passed.cir.json");
        Path log = w.resolve("stdout.txt");
        Path err = w.resolve("stderr.txt");
        run("codespec read", log, err, codespec.toString(), "read", "--lang", "java", dir.resolve("java").toString(),
            "--out", read.toString(), "--adapters", adapters.toString());
        Map<String, Object> doc = map(Json.parse(Files.readString(read)));
        if (refusedCase) {
            check(!Arrays.asList(Main.CLAIM).contains(subject), "subject " + subject + " is claimed and refused");
            List<Object> rs = doc.containsKey("refusals") ? list(doc.get("refusals")) : List.of();
            List<String> lines = Files.readAllLines(dir.resolve("java").resolve("Case.java"));
            String at = null;
            for (int i = 0; i < lines.size() && at == null; i++) {
                if (lines.get(i).strip().startsWith(REFUSED.get(subject)[0])) {
                    at = "Case.java:" + (i + 1);
                }
            }
            check(rs.size() == 1 && String.valueOf(map(rs.get(0)).get("reason")).contains(REFUSED.get(subject)[1])
                && refusalPositions(doc).contains(at), "want one refusal at " + at + ": " + Json.write(rs));
            check(foreignPositions(doc).isEmpty(), "foreign at " + foreignPositions(doc));
            return;
        }
        if (doc.containsKey("refusals")) {
            throw new AssertionError("refusals: " + Json.write(doc.get("refusals")).replaceAll("\\s+", " "));
        }
        check(foreignPositions(doc).isEmpty(), "foreign at " + foreignPositions(doc));
        String pkg = kase.replace('-', '_');
        check(pkg.equals(doc.get("name")), "module " + doc.get("name") + ", the case's package is " + pkg);
        Files.writeString(w.resolve("subject"), subject + "\n", StandardCharsets.UTF_8);
        check(Arrays.asList(Main.CLAIM).contains(subject), "subject " + subject + " is not claimed");
        run("codespec pass", log, err, codespec.toString(), "pass", read.toString(), "--out", passed.toString());
        run("codespec write --lang go", log, err, codespec.toString(), "write", "--lang", "go", passed.toString(),
            "--out", w.resolve("go").toString(), "--adapters", adapters.toString());
        Path got = w.resolve("output.txt");
        run("codespec-run-go", got, err, adapters.resolve("codespec-run-go").toString(), "-build",
            w.resolve("build").toString(), w.resolve("go").toString(), dir.resolve("calls.json").toString());
        String want = Files.readString(dir.resolve("expected.txt"));
        String have = Files.readString(got);
        check(want.equals(have), "printed\n" + have + "expected.txt\n" + want);
    }

    /**
     * Subjects whose kit case holds a Java source the reader refuses on purpose, with the start of
     * the refused line and a part of the refusal's reason: {@code decl.union}, which the reader
     * recognises but does not emit while the Go writer that checks a read refuses every union
     * ({@code Reader.READ_UNIONS}).
     */
    private static final Map<String, String[]> REFUSED = Map.of("decl.union", new String[] {"sealed interface", "a cir/1 union"});

    private void everyKitCaseWithAJavaSourcePrintsItsExpectedTxtThroughTheGoWriter() throws Exception {
        List<String> kases = javaCases();
        check(!kases.isEmpty(), "no kit case has a java/ source");
        ExecutorService pool = Executors.newFixedThreadPool(Math.min(8, Runtime.getRuntime().availableProcessors()));
        List<Future<?>> runs = new ArrayList<>();
        try {
            for (String k : kases) {
                runs.add(pool.submit(() -> {
                    kitCase(k);
                    return null;
                }));
            }
            List<String> failures = new ArrayList<>();
            for (int i = 0; i < kases.size(); i++) {
                try {
                    runs.get(i).get();
                } catch (java.util.concurrent.ExecutionException e) {
                    failures.add(kases.get(i) + ": " + e.getCause().getMessage());
                }
            }
            System.out.println("     " + (kases.size() - failures.size()) + " of " + kases.size() + " kit cases with a Java source pass");
            check(failures.isEmpty(), failures.size() + " kit case(s) failed:\n" + String.join("\n", failures));
        } finally {
            pool.shutdownNow();
        }
    }

    /**
     * Every claimed feature is the subject of at least one kit case holding {@code java/Case.java},
     * so {@code codespec conform --role reader --lang java} has a corpus case for it: a claim
     * without one fails there as {@code no_case}. A feature may be the subject of several cases;
     * one with a Java source is enough.
     */
    private void everyClaimedFeatureIsTheSubjectOfAKitCaseWithAJavaSource() throws Exception {
        Set<String> covered = new TreeSet<>();
        for (String kase : javaCases()) {
            covered.add(subject(kase));
        }
        List<String> missing = new ArrayList<>();
        for (String id : Main.CLAIM) {
            if (!covered.contains(id)) {
                missing.add(id);
            }
        }
        check(missing.isEmpty(), "claimed features that are the subject of no kit case with a java/Case.java: " + missing);
    }

    // ---- projects and Quarkus ----

    private static final String CLASSPATH_ENV = "CODESPEC_JAVA_CLASSPATH";

    /** The jars the Quarkus fixture is read against, as {@code task adapters-check} puts them in the environment. */
    private static List<Path> classpath() {
        String cp = System.getenv(CLASSPATH_ENV);
        check(cp != null && !cp.isEmpty(), CLASSPATH_ENV + " is not set: task adapters-check produces it with mvn dependency:build-classpath");
        return Reader.classpath(cp);
    }

    private Map<String, Object> quarkus() throws Exception {
        Map<String, Map<String, Object>> modules = Reader.readProject(testdata.resolve("quarkus"), classpath());
        check(modules.keySet().equals(Set.of("org.acme")), "packages " + modules.keySet());
        return modules.get("org.acme");
    }

    private void aProjectDirectoryIsReadOneModulePerPackage() throws Exception {
        Map<String, Map<String, Object>> modules = Reader.readProject(testdata.resolve("packages"), List.of());
        check(new ArrayList<>(modules.keySet()).equals(List.of("shop", "shop.pricing")), "packages " + modules.keySet());
        Map<String, Object> shop = modules.get("shop");
        Map<String, Object> pricing = modules.get("shop.pricing");
        check("shop".equals(shop.get("path")) && "shop".equals(shop.get("name")), "shop module " + shop.get("path") + " " + shop.get("name"));
        check("shop.pricing".equals(pricing.get("path")) && "pricing".equals(pricing.get("name")), "pricing module");
        check(!shop.containsKey("refusals") && !pricing.containsKey("refusals"), "refusals "
            + shop.get("refusals") + " " + pricing.get("refusals"));
        check(funcByKeyOrNull(pricing, "Price.apply") != null, "no Price.apply in the pricing module");
        String price = "{\"k\":\"extern\",\"path\":\"shop.pricing\",\"name\":\"Price\"}";
        List<Object> externs = list(shop.get("externs"));
        check(externs.size() == 1, "externs " + Json.write(externs));
        check(same(map(externs.get(0)).get("recv"), price), "extern receiver " + Json.write(map(externs.get(0)).get("recv")));
        check("apply".equals(map(externs.get(0)).get("name")) && "shop.pricing".equals(map(externs.get(0)).get("path")), "extern " + Json.write(externs));
        check(Json.write(funcByKey(shop, "Cart.total").get("body")).contains("\"k\": \"extern_method\""), "no extern_method in Cart.total");
    }

    private void aProjectOfSeveralPackagesReadWithoutNamingOneIsRefusedNamingThemAndTheVariable() throws Exception {
        Path out = work.resolve("packages.stdout.json");
        Path err = work.resolve("packages.stderr.txt");
        ProcessBuilder pb = new ProcessBuilder(adapters.resolve("codespec-read-java").toString(), testdata.resolve("packages").toString())
            .redirectOutput(out.toFile()).redirectError(err.toFile());
        pb.environment().remove("CODESPEC_JAVA_PACKAGE");
        check(pb.start().waitFor() == 1, "exit code is not 1");
        String why = Files.readString(err);
        check(why.contains("shop") && why.contains("shop.pricing") && why.contains("CODESPEC_JAVA_PACKAGE"), "stderr: " + why);
        pb.environment().put("CODESPEC_JAVA_PACKAGE", "shop.pricing");
        check(pb.start().waitFor() == 0, "exit code with CODESPEC_JAVA_PACKAGE");
        check("shop.pricing".equals(map(Json.parse(Files.readString(out))).get("path")), "module printed " + Files.readString(out));
    }

    private void aProjectReadWithoutItsJarsIsRefusedNamingTheVariable() throws Exception {
        try {
            Reader.readProject(testdata.resolve("quarkus"), List.of());
        } catch (Reader.Failure e) {
            check(e.getMessage().contains(CLASSPATH_ENV), e.getMessage());
            return;
        }
        throw new AssertionError("a project that needs its jars was read without them");
    }

    private void quarkusEntryPointsAreHttpMethodsWithJoinedPathsAndScheduledMethods() throws Exception {
        List<Object> entries = list(quarkus().get("entries"));
        List<String> got = new ArrayList<>();
        for (Object e : entries) {
            Map<String, Object> m = map(e);
            got.add(m.get("kind") + " " + m.get("func") + " " + m.getOrDefault("method", "-") + " " + m.get("on")
                + " consumes=" + m.getOrDefault("consumes", List.of()) + " produces=" + m.getOrDefault("produces", List.of()));
        }
        check(got.equals(List.of("scheduled Cleanup.sweep - every=10m consumes=[] produces=[]",
            "http FruitResource.origin GET /fruits/origin consumes=[] produces=[text/plain, text/html]",
            "http FruitResource.create POST /fruits consumes=[text/plain] produces=[application/json]")), "entries " + got);
        check(list(quarkus().get("transactional")).equals(List.of("FruitResource.create")), "transactional " + quarkus().get("transactional"));
    }

    private void quarkusBeansAndInjectionPointsResolveToTheOneBeanOrToAnExtern() throws Exception {
        Map<String, Object> doc = quarkus();
        List<String> beans = new ArrayList<>();
        for (Object b : list(doc.get("beans"))) {
            beans.add(map(b).get("name") + " " + map(b).get("scope"));
        }
        check(beans.equals(List.of("Cleanup ApplicationScoped", "FruitService ApplicationScoped")), "beans " + beans);
        List<Object> injects = list(doc.get("injects"));
        check(injects.size() == 2, "injects " + Json.write(injects));
        Map<String, Object> service = map(injects.get(0));
        check("FruitResource.service".equals(service.get("at")) && "org.acme.FruitService".equals(service.get("bean")), "service " + Json.write(service));
        check(same(service.get("t"), "{\"k\":\"ptr\",\"elem\":{\"k\":\"named\",\"name\":\"FruitService\"}}"), "service type");
        Map<String, Object> client = map(injects.get(1));
        check("FruitResource.countries".equals(client.get("at")) && !client.containsKey("bean"), "client " + Json.write(client));
        check(same(client.get("t"), "{\"k\":\"extern\",\"path\":\"org.acme.CountryClient\",\"name\":\"CountryClient\"}"), "client type");
    }

    private void aCallIntoAJarIsAnExternWithItsDeclaredSignature() throws Exception {
        Map<String, Object> doc = quarkus();
        List<String> got = new ArrayList<>();
        for (Object x : list(doc.get("externs"))) {
            Map<String, Object> m = map(x);
            got.add(m.get("path") + " " + (m.containsKey("recv") ? map(map(m.get("recv")).get("k").equals("ptr") ? map(m.get("recv")).get("elem") : m.get("recv")).get("name") : "-")
                + " " + m.get("name") + " " + Json.write(m.get("params")).replaceAll("\\s+", "") + Json.write(m.get("results")).replaceAll("\\s+", ""));
        }
        String s = "{\"k\":\"string\"}";
        check(got.equals(List.of(
            "io.quarkus.hibernate.orm.panache.PanacheEntityBase - deleteAll []" + "[{\"k\":\"int\",\"width\":\"64\",\"signed\":true}]",
            "io.quarkus.hibernate.orm.panache.PanacheEntityBase PanacheEntityBase persist [][]",
            "org.acme.CountryClient CountryClient byName [" + s + "][" + s + "]")), "externs " + got);
    }

    private void quarkusRefusesOnlyWhatItCannotMap() throws Exception {
        Map<String, Object> doc = quarkus();
        List<Object> refusals = list(doc.get("refusals"));
        check(refusals.size() == 1, "refusals " + Json.write(refusals).replaceAll("\\s+", " "));
        check(String.valueOf(map(refusals.get(0)).get("reason")).startsWith("annotation jakarta.persistence.Entity"), Json.write(refusals));
        check("org.acme".equals(doc.get("path")), "path " + doc.get("path"));
    }

    private void quarkusReadTwiceGivesTheSameBytes() throws Exception {
        check(Json.write(quarkus()).equals(Json.write(quarkus())), "two reads differ");
    }

    /** {@code codespec effects} over the read, with the summary the reader prints for the project. */
    private void effectsOverTheQuarkusReadGiveThePersistingMethodDbAndTheClientMethodNet() throws Exception {
        Path read = work.resolve("quarkus.read.json");
        Path summary = work.resolve("quarkus.effects.json");
        Path report = work.resolve("quarkus.report.json");
        Files.writeString(read, Json.write(quarkus()), StandardCharsets.UTF_8);
        Files.writeString(summary, Reader.effectsSummary(testdata.resolve("quarkus"), classpath()), StandardCharsets.UTF_8);
        run("codespec effects", report, work.resolve("quarkus.effects.err"), codespec.toString(), "effects", read.toString(),
            "--summaries", summary.toString());
        Map<String, Object> doc = map(Json.parse(Files.readString(report)));
        check(Json.write(map(funcByKeyInReport(doc, "FruitResource.create")).get("effects")).replaceAll("\\s+", "").equals("[\"db\"]"),
            "create " + Json.write(funcByKeyInReport(doc, "FruitResource.create")));
        check(Json.write(map(funcByKeyInReport(doc, "FruitResource.origin")).get("effects")).replaceAll("\\s+", "").equals("[\"net\"]"),
            "origin " + Json.write(funcByKeyInReport(doc, "FruitResource.origin")));
        check(Json.write(map(funcByKeyInReport(doc, "Cleanup.sweep")).get("effects")).replaceAll("\\s+", "").equals("[\"db\"]"),
            "sweep " + Json.write(funcByKeyInReport(doc, "Cleanup.sweep")));
        check(Json.write(map(funcByKeyInReport(doc, "FruitService.defaultName")).get("effects")).replaceAll("\\s+", "").equals("[]"), "defaultName has effects");
        check(Json.write(map(funcByKeyInReport(doc, "FruitResource.create")).get("unknown")).replaceAll("\\s+", "").equals("[]"), "create is unknown: "
            + Json.write(funcByKeyInReport(doc, "FruitResource.create")));
    }

    private static Map<String, Object> funcByKeyInReport(Map<String, Object> report, String key) {
        for (Object f : list(report.get("funcs"))) {
            if (key.equals(map(f).get("name"))) {
                return map(f);
            }
        }
        throw new AssertionError("no " + key + " in the report");
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

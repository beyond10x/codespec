package codespec.read.groovy;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

/**
 * The Groovy reader's tests, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt;:&lt;groovy-all jar&gt; codespec.read.groovy.ReaderTest &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt;</pre>
 *
 * It prints one line per case and a summary {@code N passed, M failed, K skipped}, and exits 1 when any case
 * fails.
 *
 * <p>Each fixture under {@code testdata/refuse/} marks one line {@code // refused <category>}. The
 * reader must refuse at that line with a reason beginning {@code <category>: }; where the line is a
 * statement, it must also be a {@code foreign} statement there. A declaration (a method, an import,
 * a property) has no foreign form, so for those the refusal alone is checked, and the test checks
 * that the declaration was not read.
 */
public final class ReaderTest {
    /** The closed list of refusal categories, in the order the README lists them. */
    static final List<String> CATEGORIES = List.of(
        "dynamic", "closure", "groovy-semantics", "type", "unresolved", "gorm", "spring", "unsupported");

    private static final String INT32 = "{\"k\":\"int\",\"width\":\"32\",\"signed\":true}";
    private static final String INT64 = "{\"k\":\"int\",\"width\":\"64\",\"signed\":true}";

    private final Path root;
    private final Path codespec;
    private final Path work;
    private final Path testdata;

    private ReaderTest(Path root, Path codespec, Path work) {
        this.root = root;
        this.codespec = codespec;
        this.work = work;
        this.testdata = root.resolve("adapters/groovy/testdata");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 3 && args.length != 4) {
            System.err.println("usage: ReaderTest <repo root> <codespec binary> <work dir> [<case name substring>]");
            System.exit(2);
        }
        ReaderTest t = new ReaderTest(Path.of(args[0]), Path.of(args[1]), Path.of(args[2]));
        Files.createDirectories(t.work);
        Map<String, Case> cases = new LinkedHashMap<>();
        cases.put("typed_reads_with_no_refusal", t::typedReadsWithNoRefusal);
        cases.put("typed_read_twice_gives_the_same_bytes", t::typedReadTwiceGivesTheSameBytes);
        cases.put("typed_document_is_canonical_cir_1", t::typedDocumentIsCanonical);
        cases.put("typed_holds_every_construct_of_the_story", t::typedHoldsEveryConstruct);
        cases.put("after_the_passes_the_hash_map_loop_is_unordered_and_the_tree_map_loop_is_not", t::hashMapLoopIsUnordered);
        cases.put("integer_max_value_plus_one_is_int32_and_a_long_field_int64", t::integerMaxValuePlusOneIsInt32);
        cases.put("a_def_local_is_typed_from_its_inferred_type", t::aDefLocalIsTypedFromItsInferredType);
        cases.put("the_compile_static_class_and_the_unannotated_class_are_both_read", t::bothClassesAreRead);
        cases.put("the_reader_version_is_the_workspace_version", t::theReaderVersionIsTheWorkspaceVersion);
        cases.put("the_manifest_claims_only_features_the_typed_read_uses", t::theManifestClaimsOnlyUsedFeatures);
        cases.put("a_def_parameter_called_dynamically_is_foreign_dynamic", () -> t.foreignAt("def-param"));
        cases.put("a_method_missing_declared_is_refused_dynamic", () -> t.declarationRefused("method-missing", "Desk.methodMissing"));
        cases.put("a_meta_class_assignment_is_foreign_dynamic", () -> t.foreignAt("meta-class"));
        cases.put("a_closure_passed_to_each_is_foreign_closure", () -> t.foreignAt("closure-each"));
        cases.put("groovy_truth_in_an_if_is_foreign_groovy_semantics", () -> t.foreignAt("groovy-truth"));
        cases.put("safe_navigation_is_foreign_groovy_semantics", () -> t.foreignAt("safe-navigation"));
        cases.put("the_elvis_operator_is_foreign_groovy_semantics", () -> t.foreignAt("elvis"));
        cases.put("int_divided_by_int_is_foreign_groovy_semantics", () -> t.foreignAt("int-division"));
        cases.put("intdiv_of_two_longs_or_two_ints_reads_as_op_div_of_their_type", t::intdivReadsAsOpDiv);
        cases.put("x_assigned_x_intdiv_y_reads_as_assign_div_and_y_assigned_x_intdiv_z_as_an_assignment_of_op_div",
            t::intdivAssignmentReadsAsAssignDiv);
        cases.put("intdiv_of_an_int_and_a_long_is_foreign_groovy_semantics", () -> t.foreignAt("intdiv-mixed"));
        cases.put("intdiv_of_an_int_literal_over_a_long_reads_as_int64_op_div", t::intdivOfALiteralReceiverIsInt64);
        cases.put("intdiv_of_two_shorts_is_foreign_groovy_semantics", () -> t.foreignAt("intdiv-short"));
        cases.put("a_field_assigned_its_own_intdiv_through_no_call_reads_as_assign_div_and_through_a_call_as_an_assignment",
            t::intdivFieldTargets);
        cases.put("groovy_intdiv_truncates_toward_zero_wraps_and_throws_on_zero_as_go_divides",
            ReaderTest::groovyIntdivDividesAsGo);
        cases.put("adversary_intdiv_as_an_expression_statement_is_foreign_unsupported_as_a_plus_b_is",
            () -> t.foreignAt("intdiv-statement"));
        cases.put("adversary_intdiv_assigned_to_a_long_in_a_compile_static_class_is_foreign_as_groovyc_refuses_it",
            () -> t.foreignAt("intdiv-compile-static"));
        cases.put("a_default_parameter_value_is_refused_groovy_semantics", () -> t.declarationRefused("default-param", "Desk.scaled"));
        cases.put("a_big_decimal_literal_is_foreign_type_with_the_shared_text", t::aBigDecimalLiteralIsForeignType);
        cases.put("an_unresolved_import_refuses_its_file_and_the_rest_is_read", t::anUnresolvedImportRefusesItsFile);
        cases.put("a_string_property_without_an_initializer_is_refused_type", t::aStringPropertyIsRefusedType);
        cases.put("every_refusal_of_every_fixture_begins_with_a_category", t::everyRefusalBeginsWithACategory);
        cases.put("the_readme_lists_the_eight_categories_each_with_an_example", t::theReadmeListsTheCategories);
        // Adversary: Groovy code the reader must not read into cir/1 with a meaning Groovy does not
        // give it. Each fixture under testdata/adversary marks with "// diverges" the lines whose
        // read differs from a Groovy 2.4.10 run; each case passes when the reader refuses one of
        // them, or reads none of them in the way named.
        cases.put("string_equals_of_a_gstring_is_not_read_as_string_equality",
            () -> t.notReadAs("string-equals-gstring", s -> Json.write(s).contains("\"op\": \"==\"")));
        cases.put("a_property_write_through_a_declared_setter_is_not_read_as_a_field_write",
            () -> t.notReadAs("setter-dynamic", ReaderTest::writesFieldCopies));
        cases.put("a_compile_static_property_write_through_a_declared_setter_is_not_read_as_a_field_write",
            () -> t.notReadAs("setter-compile-static", ReaderTest::writesFieldCopies));
        cases.put("a_write_to_a_final_property_which_groovy_refuses_at_run_time_is_not_read_as_a_field_write",
            () -> t.notReadAs("final-property", ReaderTest::writesFieldCopies));
        cases.put("a_property_of_a_class_declaring_get_property_or_set_property_is_not_read_as_its_field",
            () -> t.notReadAs("property-hooks", s -> Json.write(s).contains("\"name\": \"copies\"")));
        cases.put("a_def_local_is_not_read_as_long_before_groovy_widens_it",
            () -> t.notReadAs("def-widened", s -> Json.write(s).contains("\"width\": \"64\"")));
        cases.put("a_list_grown_through_a_method_while_a_for_each_iterates_it_is_not_read_as_a_loop_that_runs_on",
            () -> t.notReadAs("modified-while-iterated", s -> true));
        // Grails: the lending application under testdata/lending.
        cases.put("lending_read_matches_expected_md_and_the_markers_per_artefact", t::lendingMatchesExpected);
        cases.put("lending_read_twice_gives_the_same_canonical_bytes", t::lendingReadTwiceIsCanonical);
        cases.put("lending_read_equals_the_checked_in_document", t::lendingEqualsTheCheckedInDocument);
        cases.put("lending_resolve_only_files_yield_nothing_and_due_dates_resolves", t::lendingResolveOnlyYieldsNothing);
        cases.put("lending_a_tool_string_property_is_a_pointer_with_zero_value_nil", t::lendingToolNameIsAPointer);
        cases.put("lending_find_by_name_and_save_are_foreign_gorm_at_their_lines", t::lendingGormCallsAreForeign);
        cases.put("lending_spring_injected_fields_and_calls_through_them_are_refused_spring", t::lendingSpringInjection);
        cases.put("lending_domain_metadata_is_refused_gorm_mapping_metadata", t::lendingDomainMetadata);
        cases.put("lending_without_the_grails_jars_the_transactional_service_is_refused_unresolved_at_its_import",
            t::lendingTransactionalUnresolvedWithoutJars);
        cases.put("lending_every_reason_begins_with_a_category", t::lendingEveryReasonHasACategory);
        cases.put("a_directory_without_application_properties_reads_as_plain_groovy", t::plainDirectoryReadsAsGroovy);
        cases.put("two_artefacts_with_one_simple_name_are_both_refused", t::duplicateSimpleNamesAreRefused);
        cases.put("the_readme_names_the_grails_jars_as_grails_home_paths", t::theReadmeNamesTheGrailsJars);
        cases.put("lending_with_the_grails_jars_the_transactional_service_reads", t::lendingTransactionalWithJars);
        cases.put("lending_with_the_grails_jars_no_identifier_of_the_transactional_transform_is_in_the_read",
            t::lendingNoTransformIdentifier);
        // Adversary (story:grails-artefacts): Grails 2.5.6 code read wrong, or refused for a reason
        // that is not why it cannot be read. Each case builds a small application under the work dir.
        cases.put("adversary_grails_a_service_injected_grails_application_is_spring_and_its_other_methods_are_read",
            t::adversaryGrailsApplicationInjected);
        cases.put("adversary_grails_a_domain_string_property_written_or_compared_with_null_is_read_as_the_java_reader_reads_it",
            t::adversaryNullablePropertyUses);
        cases.put("adversary_grails_a_service_naming_a_src_java_class_is_not_refused_unresolved",
            t::adversarySrcJavaResolves);
        cases.put("adversary_grails_with_the_grails_jars_an_entity_class_in_src_groovy_does_not_fail_the_read",
            t::adversaryEntityInSrcGroovy);
        cases.put("adversary_grails_an_initialised_service_field_that_spring_replaces_is_not_read_as_the_services_own_instance",
            t::adversaryInitialisedServiceField);
        cases.put("adversary_grails_with_the_grails_jars_an_overload_the_transform_did_not_rewrite_is_not_refused_as_rewritten",
            t::adversaryOverloadNotRewritten);
        cases.put("adversary_grails_list_order_by_on_a_domain_class_is_foreign_gorm", t::adversaryListOrderBy);
        String only = args.length == 4 ? args[3] : null;
        if (only != null) {
            cases.keySet().removeIf(k -> !k.contains(only));
        }
        int passed = 0;
        int failed = 0;
        int skipped = 0;
        for (Map.Entry<String, Case> c : cases.entrySet()) {
            try {
                c.getValue().run();
                System.out.println("ok   " + c.getKey());
                passed++;
            } catch (Skip s) {
                System.out.println("skip " + c.getKey() + ": " + s.getMessage());
                skipped++;
            } catch (Throwable e) {
                System.out.println("FAIL " + c.getKey() + ": " + e);
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed, " + skipped + " skipped");
        System.exit(failed == 0 ? 0 : 1);
    }

    // ---- typed ----

    private Map<String, Object> typed() throws Exception {
        return Reader.read(testdata.resolve("typed"));
    }

    private void typedReadsWithNoRefusal() throws Exception {
        Map<String, Object> doc = typed();
        check(!doc.containsKey("refusals"), "refusals: " + Json.write(doc.getOrDefault("refusals", List.of())));
        check("cir/1".equals(doc.get("format")), "format");
        check("typed".equals(doc.get("name")), "name " + doc.get("name"));
        Map<String, Object> reader = map(doc.get("reader"));
        check("groovy".equals(reader.get("language")), "reader.language");
        check("codespec-read-groovy".equals(reader.get("name")), "reader.name");
        check(foreignPositions(doc.get("funcs")).isEmpty(), "foreign statements at " + foreignPositions(doc.get("funcs")));
    }

    private void typedReadTwiceGivesTheSameBytes() throws Exception {
        String a = Json.write(typed());
        String b = Json.write(typed());
        check(a.equals(b), "two reads differ");
    }

    /** The adapter's own bytes, before the CLI re-encodes them, are canonical cir/1. */
    private void typedDocumentIsCanonical() throws Exception {
        Path doc = work.resolve("typed.raw.json");
        Files.writeString(doc, Json.write(typed()), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "validate", "--canonical", doc.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec validate --canonical failed");
    }

    private void typedHoldsEveryConstruct() throws Exception {
        Map<String, Object> doc = typed();
        Map<String, Object> kind = byName(list(doc.get("types")), "Kind");
        check("enum".equals(kind.get("k")) && list(kind.get("variants")).size() == 2, "Kind " + Json.write(kind));
        Map<String, Object> tally = byName(list(doc.get("types")), "Tally");
        check("struct".equals(tally.get("k")), "Tally is " + tally.get("k"));
        String funcs = Json.write(doc.get("funcs"));
        for (String needle : new String[] {
            "\"k\": \"switch\"", "\"k\": \"for\"", "\"k\": \"break\"", "\"k\": \"for_range\"", "\"name\": \"sprintf\"",
            "\"name\": \"append\"", "\"name\": \"sorted_keys\"", "\"k\": \"incr\"",
        }) {
            check(funcs.contains(needle), "no " + needle + " in funcs");
        }
        Object describe = returned(funcByKey(doc, "describe"));
        check(same(describe, "{\"k\":\"intrinsic\",\"name\":\"sprintf\",\"args\":["
            + "{\"k\":\"lit\",\"value\":\"\\\"%s has %d\\\"\",\"t\":{\"k\":\"string\"}},"
            + "{\"k\":\"var\",\"name\":\"name\",\"t\":{\"k\":\"string\"}},"
            + "{\"k\":\"var\",\"name\":\"count\",\"t\":" + INT32 + "}],\"t\":{\"k\":\"string\"}}"),
            "describe returns " + Json.write(describe));
    }

    private void hashMapLoopIsUnordered() throws Exception {
        Path read = work.resolve("typed.read.json");
        Path passed = work.resolve("typed.pass.json");
        Files.writeString(read, Json.write(typed()), StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "pass", read.toString(), "--out", passed.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec pass failed");
        Map<String, Object> facts = map(map(Json.parse(Files.readString(passed))).get("facts"));
        List<Object> unordered = list(facts.get("unordered_iteration"));
        Path tally = testdata.resolve("typed/Tally.groovy");
        String hashLoop = "Tally.groovy:" + lineOf(tally, "for (String k : counts.keySet())");
        String treeLoop = "Tally.groovy:" + lineOf(tally, "for (String k : sorted.keySet())");
        check(unordered.contains(hashLoop), "unordered_iteration " + unordered + " lacks the HashMap loop " + hashLoop);
        check(!unordered.contains(treeLoop), "unordered_iteration " + unordered + " holds the TreeMap loop " + treeLoop);
    }

    private void integerMaxValuePlusOneIsInt32() throws Exception {
        Map<String, Object> doc = typed();
        Object wrap = returned(funcByKey(doc, "wrap"));
        check(same(wrap, "{\"k\":\"binop\",\"op\":\"+\",\"x\":{\"k\":\"lit\",\"value\":\"2147483647\",\"t\":" + INT32
            + "},\"y\":{\"k\":\"lit\",\"value\":\"1\",\"t\":" + INT32 + "},\"t\":" + INT32 + "}"), "wrap returns " + Json.write(wrap));
        Map<String, Object> tally = byName(list(doc.get("types")), "Tally");
        check(same(byName(list(tally.get("fields")), "total").get("t"), INT64), "Tally.total is not int64");
        check(same(byName(list(tally.get("fields")), "counter").get("t"), INT32), "Tally.counter is not int32");
    }

    private void aDefLocalIsTypedFromItsInferredType() throws Exception {
        List<Object> body = list(funcByKey(typed(), "doubled").get("body"));
        Map<String, Object> let = map(body.get(0));
        check("let".equals(let.get("k")) && List.of("twice").equals(let.get("names")), "first statement " + Json.write(let));
        check(same(map(list(let.get("rhs")).get(0)).get("t"), INT32), "twice is not int32: " + Json.write(let));
    }

    private void bothClassesAreRead() throws Exception {
        Map<String, Object> doc = typed();
        check(funcByKeyOrNull(doc, "firstOver") != null, "Label.firstOver (@CompileStatic) was not read");
        check(funcByKeyOrNull(doc, "Tally.countOf") != null, "Tally.countOf (unannotated) was not read");
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
        check(Main.VERSION.equals(map(typed().get("reader")).get("version")), "reader.version");
    }

    /** The manifest is the reader's own JSON; its claim is checked against the read by codespec-conformance. */
    private void theManifestClaimsOnlyUsedFeatures() throws Exception {
        Map<String, Object> m = map(Json.parse(Main.describe()));
        check(Main.describe().equals(Json.write(m)), "the manifest is not in canonical layout");
        check("codespec-adapter/1".equals(m.get("format")) && "reader".equals(m.get("role")) && "groovy".equals(m.get("lang")),
            "manifest head " + Json.write(m));
        for (Object f : list(m.get("features"))) {
            check(!((String) f).startsWith("fact."), "a reader claims " + f);
        }
    }

    // ---- refusals ----

    /** {@code <file>:<line>} and category of the one line marked {@code // refused <category>} in a fixture. */
    private String[] marker(String fixture) throws IOException {
        List<String[]> found = new ArrayList<>();
        try (Stream<Path> files = Files.list(testdata.resolve("refuse/" + fixture))) {
            for (Path f : files.sorted().toList()) {
                List<String> lines = Files.readAllLines(f);
                for (int i = 0; i < lines.size(); i++) {
                    int at = lines.get(i).indexOf("// refused ");
                    if (at >= 0) {
                        found.add(new String[] {f.getFileName() + ":" + (i + 1), lines.get(i).substring(at + "// refused ".length()).trim()});
                    }
                }
            }
        }
        check(found.size() == 1, "markers in " + fixture + ": " + found.size());
        return found.get(0);
    }

    private Map<String, Object> refusalAtMarker(String fixture) throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/" + fixture));
        String[] m = marker(fixture);
        List<String> at = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            at.add(rm.get("pos") + " " + rm.get("reason"));
            if (m[0].equals(rm.get("pos")) && ((String) rm.get("reason")).startsWith(m[1] + ": ")) {
                return doc;
            }
        }
        throw new AssertionError("no refusal " + m[1] + " at " + m[0] + "; refusals " + at);
    }

    private void foreignAt(String fixture) throws Exception {
        Map<String, Object> doc = refusalAtMarker(fixture);
        String pos = marker(fixture)[0];
        check(foreignPositions(doc.get("funcs")).contains(pos), "no foreign statement at " + pos);
    }

    private void declarationRefused(String fixture, String key) throws Exception {
        Map<String, Object> doc = refusalAtMarker(fixture);
        check(funcByKeyOrNull(doc, key) == null, key + " was read");
        check(funcByKeyOrNull(doc, "Desk.one") != null, "the method beside it was not read");
    }

    private void aBigDecimalLiteralIsForeignType() throws Exception {
        foreignAt("big-decimal");
        Map<String, Object> doc = Reader.read(testdata.resolve("refuse/big-decimal"));
        String reason = (String) map(list(doc.get("refusals")).get(0)).get("reason");
        check(reason.equals("type: type java.math.BigDecimal is outside the reader's types"), "reason " + reason);
    }

    // ---- intdiv ----

    private static final String INTDIV = "package intdiv\n\nfinal class Case {\n"
        + "    static long div(long a, long b) {\n        return a.intdiv(b)\n    }\n\n"
        + "    static int div32(int a, int b) {\n        return a.intdiv(b)\n    }\n\n"
        + "    static long divBy(long a, long b) {\n        a = a.intdiv(b)\n        return a\n    }\n\n"
        + "    static long quotient(long a, long b) {\n        long q = 0\n        q = a.intdiv(b)\n        return q\n    }\n\n"
        + "    static long seven(long b) {\n        return 7.intdiv(b)\n    }\n}\n\n"
        + "final class Till {\n    long total = 0\n\n"
        + "    void halve(long b) {\n        total = total.intdiv(b)\n    }\n\n"
        + "    void halveOther(Till o, long b) {\n        o.total = o.total.intdiv(b)\n    }\n\n"
        + "    Till self() {\n        return this\n    }\n\n"
        + "    void halveThroughCall(long b) {\n        self().total = self().total.intdiv(b)\n    }\n}\n";

    private Map<String, Object> intdiv() throws Exception {
        Path dir = work.resolve("intdiv");
        write(dir.resolve("Case.groovy"), INTDIV);
        Map<String, Object> doc = Reader.read(dir);
        check(!doc.containsKey("refusals"), "refusals: " + Json.write(doc.getOrDefault("refusals", List.of())));
        return doc;
    }

    private static String var(String name, String t) {
        return "{\"k\":\"var\",\"name\":\"" + name + "\",\"t\":" + t + "}";
    }

    private void intdivReadsAsOpDiv() throws Exception {
        Map<String, Object> doc = intdiv();
        Object div = returned(funcByKey(doc, "div"));
        check(same(div, "{\"k\":\"binop\",\"op\":\"/\",\"x\":" + var("a", INT64) + ",\"y\":" + var("b", INT64) + ",\"t\":" + INT64 + "}"),
            "div returns " + Json.write(div));
        Object div32 = returned(funcByKey(doc, "div32"));
        check(same(div32, "{\"k\":\"binop\",\"op\":\"/\",\"x\":" + var("a", INT32) + ",\"y\":" + var("b", INT32) + ",\"t\":" + INT32 + "}"),
            "div32 returns " + Json.write(div32));
    }

    private void intdivAssignmentReadsAsAssignDiv() throws Exception {
        Map<String, Object> doc = intdiv();
        Map<String, Object> divBy = map(list(funcByKey(doc, "divBy").get("body")).get(0));
        check("assign".equals(divBy.get("k")) && "/=".equals(divBy.get("op")), "divBy's first statement " + Json.write(divBy));
        check(same(divBy.get("lhs"), "[" + var("a", INT64) + "]") && same(divBy.get("rhs"), "[" + var("b", INT64) + "]"),
            "divBy's first statement " + Json.write(divBy));
        Map<String, Object> q = map(list(funcByKey(doc, "quotient").get("body")).get(1));
        check("assign".equals(q.get("k")) && !q.containsKey("op"), "quotient's second statement " + Json.write(q));
        check(same(q.get("rhs"), "[{\"k\":\"binop\",\"op\":\"/\",\"x\":" + var("a", INT64) + ",\"y\":" + var("b", INT64)
            + ",\"t\":" + INT64 + "}]"), "quotient's second statement " + Json.write(q));
    }

    /** {@code 7.intdiv(b)}, an int literal over a long: Groovy divides in long, so the literal is int64. */
    private void intdivOfALiteralReceiverIsInt64() throws Exception {
        Object seven = returned(funcByKey(intdiv(), "seven"));
        check(same(seven, "{\"k\":\"binop\",\"op\":\"/\",\"x\":{\"k\":\"lit\",\"value\":\"7\",\"t\":" + INT64 + "},\"y\":"
            + var("b", INT64) + ",\"t\":" + INT64 + "}"), "seven returns " + Json.write(seven));
    }

    /**
     * A field target read through no call, {@code this} or a local, is {@code /=}; one read through a
     * call, evaluated twice by Groovy, is an assignment of the division.
     */
    private void intdivFieldTargets() throws Exception {
        Map<String, Object> doc = intdiv();
        for (String fn : List.of("Till.halve", "Till.halveOther")) {
            Map<String, Object> st = map(list(funcByKey(doc, fn).get("body")).get(0));
            check("assign".equals(st.get("k")) && "/=".equals(st.get("op")), fn + "'s statement " + Json.write(st));
            check("field".equals(map(list(st.get("lhs")).get(0)).get("k")), fn + "'s target " + Json.write(st));
            check(same(st.get("rhs"), "[" + var("b", INT64) + "]"), fn + "'s statement " + Json.write(st));
        }
        Map<String, Object> st = map(list(funcByKey(doc, "Till.halveThroughCall").get("body")).get(0));
        check("assign".equals(st.get("k")) && !st.containsKey("op"), "halveThroughCall's statement " + Json.write(st));
        check("binop".equals(map(list(st.get("rhs")).get(0)).get("k")), "halveThroughCall's statement " + Json.write(st));
    }

    /**
     * What Groovy 2.4.10 itself gives for {@code intdiv}, run through {@code GroovyShell}: Go's
     * integer division of one type, truncating toward zero, the minimum divided by -1 wrapping to
     * itself, and division by zero throwing where Go panics.
     */
    private static void groovyIntdivDividesAsGo() {
        groovy.lang.GroovyShell sh = new groovy.lang.GroovyShell();
        check(Long.valueOf(-3).equals(sh.evaluate("(-7L).intdiv(2L)")), "(-7L).intdiv(2L)");
        check(Long.valueOf(3).equals(sh.evaluate("7L.intdiv(2L)")), "7L.intdiv(2L)");
        check(Integer.valueOf(-3).equals(sh.evaluate("(-7).intdiv(2)")), "(-7).intdiv(2) is not the int -3");
        check(Long.valueOf(Long.MIN_VALUE).equals(sh.evaluate("Long.MIN_VALUE.intdiv(-1L)")), "Long.MIN_VALUE.intdiv(-1L)");
        check(Integer.valueOf(Integer.MIN_VALUE).equals(sh.evaluate("Integer.MIN_VALUE.intdiv(-1)")), "Integer.MIN_VALUE.intdiv(-1)");
        for (String zero : List.of("7L.intdiv(0L)", "7.intdiv(0)")) {
            try {
                sh.evaluate(zero);
                check(false, zero + " did not throw");
            } catch (ArithmeticException e) {
                // Go panics: integer divide by zero
            }
        }
    }

    private void anUnresolvedImportRefusesItsFile() throws Exception {
        Map<String, Object> doc = refusalAtMarker("unresolved-import");
        check(list(doc.get("refusals")).size() == 1, "refusals " + Json.write(doc.get("refusals")));
        String reason = (String) map(list(doc.get("refusals")).get(0)).get("reason");
        check(reason.equals("unresolved: com.example.missing.Gone"), "reason " + reason);
        check(byNameOrNull(list(doc.get("types")), "Desk") == null, "Desk, whose import does not resolve, was read");
        check(funcByKeyOrNull(doc, "Shelf.two") != null, "Shelf, beside it, was not read");
    }

    private void aStringPropertyIsRefusedType() throws Exception {
        Map<String, Object> doc = refusalAtMarker("string-property");
        String reason = (String) map(list(doc.get("refusals")).get(0)).get("reason");
        check(reason.contains("title") && reason.contains("null until assigned"), "reason " + reason);
        check(byNameOrNull(list(doc.get("types")), "Book") == null, "Book was read with a zero value where Groovy holds null");
    }

    private void everyRefusalBeginsWithACategory() throws Exception {
        List<String> bad = new ArrayList<>();
        try (Stream<Path> dirs = Files.list(testdata.resolve("refuse"))) {
            for (Path d : dirs.sorted().toList()) {
                Map<String, Object> doc = Reader.read(d);
                for (Object r : list(doc.get("refusals"))) {
                    String reason = (String) map(r).get("reason");
                    int colon = reason.indexOf(": ");
                    if (colon < 0 || !CATEGORIES.contains(reason.substring(0, colon))) {
                        bad.add(d.getFileName() + " " + map(r).get("pos") + " " + reason);
                    }
                }
                for (String reason : foreignReasons(doc.get("funcs"))) {
                    int colon = reason.indexOf(": ");
                    if (colon < 0 || !CATEGORIES.contains(reason.substring(0, colon))) {
                        bad.add(d.getFileName() + " foreign " + reason);
                    }
                }
            }
        }
        check(bad.isEmpty(), "reasons without a category: " + bad);
    }

    /** README.md has a {@code ### `<category>`} section for each category, each holding a {@code groovy} example. */
    private void theReadmeListsTheCategories() throws Exception {
        String readme = Files.readString(root.resolve("adapters/groovy/README.md"));
        List<String> missing = new ArrayList<>();
        for (String c : CATEGORIES) {
            String head = "### `" + c + "`";
            int at = readme.indexOf(head);
            if (at < 0) {
                missing.add(c + " (no section)");
                continue;
            }
            int next = readme.indexOf("\n#", at + head.length());
            String section = readme.substring(at, next < 0 ? readme.length() : next);
            if (!section.contains("```groovy")) {
                missing.add(c + " (no example)");
            }
        }
        check(missing.isEmpty(), "README.md lacks " + missing);
    }

    // ---- Grails: the lending application ----

    /** A case that cannot run here; the summary counts it as skipped, saying why. */
    static final class Skip extends RuntimeException {
        private static final long serialVersionUID = 1L;

        Skip(String why) {
            super(why, null, false, false);
        }
    }

    /**
     * The Grails 2.5.6 distribution jars a Grails application is read with, relative to {@code
     * GRAILS_HOME}: {@code grails.transaction.Transactional} and its transform (grails-core), the
     * Spring enums its members name (spring-tx), and the classes the transform loads as it runs
     * (jsr250-api, spring-beans, spring-core, grails-bootstrap). README.md names each.
     */
    static final List<String> GRAILS_JARS = List.of(
        "dist/grails-core-2.5.6.jar",
        "lib/org.springframework/spring-tx/jars/spring-tx-4.1.9.RELEASE.jar",
        "lib/javax.annotation/jsr250-api/jars/jsr250-api-1.0.jar",
        "lib/org.springframework/spring-beans/jars/spring-beans-4.1.9.RELEASE.jar",
        "lib/org.springframework/spring-core/jars/spring-core-4.1.9.RELEASE.jar",
        "dist/grails-bootstrap-2.5.6.jar");

    /** The identifiers {@code TransactionalTransform} introduces, which no read may hold. */
    static final List<String> TRANSFORM_IDENTIFIERS = List.of(
        "$tt__", "transactionManager", "transactionStatus", "GrailsTransactionTemplate", "PlatformTransactionManager");

    private static final String AUDIT = "grails-app/services/lending/TransactionalAuditService.groovy";
    private static final String LOAN_SERVICE = "grails-app/services/lending/LoanService.groovy";

    private Path lendingDir() {
        return testdata.resolve("lending");
    }

    private Map<String, Object> lending() throws Exception {
        return Main.read(lendingDir(), List.of());
    }

    private List<Path> grailsJars() {
        String home = System.getenv("GRAILS_HOME");
        if (home == null || home.isEmpty()) {
            throw new Skip("GRAILS_HOME is not set; set it to a Grails 2.5.6 distribution to run this case");
        }
        List<Path> out = new ArrayList<>();
        for (String j : GRAILS_JARS) {
            Path p = Path.of(home).resolve(j);
            check(Files.isRegularFile(p), "GRAILS_HOME has no " + j);
            out.add(p);
        }
        return out;
    }

    private void lendingMatchesExpected() throws Exception {
        List<String> problems = Lending.compare(lending(), lendingDir());
        check(problems.isEmpty(), "the lending read differs from the fixture:\n  " + String.join("\n  ", problems));
    }

    private void lendingReadTwiceIsCanonical() throws Exception {
        String a = Json.write(lending());
        String b = Json.write(lending());
        check(a.equals(b), "two reads of lending differ");
        Path doc = work.resolve("lending.raw.json");
        Files.writeString(doc, a, StandardCharsets.UTF_8);
        Process p = new ProcessBuilder(codespec.toString(), "validate", "--canonical", doc.toString()).inheritIO().start();
        check(p.waitFor() == 0, "codespec validate --canonical failed on the lending read");
    }

    private void lendingEqualsTheCheckedInDocument() throws Exception {
        Path checkedIn = testdata.resolve("lending.read.cir.json");
        check(Files.isRegularFile(checkedIn), "no " + checkedIn);
        check(Json.write(lending()).equals(Files.readString(checkedIn)), "the lending read differs from " + checkedIn
            + "; regenerate it with task examples");
    }

    private void lendingResolveOnlyYieldsNothing() throws Exception {
        Map<String, Object> doc = lending();
        List<String> found = new ArrayList<>();
        for (Map<String, Object> s : statements(doc)) {
            String pos = (String) s.get("pos");
            if (pos != null && (pos.startsWith("grails-app/controllers/") || pos.startsWith("src/")
                || pos.startsWith("grails-app/utils/") || pos.startsWith("grails-app/conf/"))) {
                found.add(pos);
            }
        }
        for (Object r : list(doc.get("refusals"))) {
            String pos = (String) map(r).get("pos");
            if (!pos.startsWith("grails-app/domain/") && !pos.startsWith("grails-app/services/")) {
                found.add(pos);
            }
        }
        for (String name : List.of("ToolController", "ShelfCodes", "DueDates")) {
            if (byNameOrNull(list(doc.get("types")), name) != null) {
                found.add("type " + name);
            }
        }
        check(found.isEmpty(), "read from a resolve-only file: " + found);
        int line = lineOf(lendingDir().resolve(LOAN_SERVICE), "DueDates.dueDay(");
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            if ((LOAN_SERVICE + ":" + line).equals(rm.get("pos"))) {
                check(!((String) rm.get("reason")).startsWith("unresolved"), "the DueDates call is refused " + rm.get("reason"));
            }
        }
        check(funcByKeyOrNull(doc, "LoanService.dueDayFor") != null, "LoanService.dueDayFor was not read as a function");
    }

    private void lendingToolNameIsAPointer() throws Exception {
        Map<String, Object> doc = lending();
        String ptrString = "{\"k\":\"ptr\",\"elem\":{\"k\":\"string\"}}";
        for (String[] sf : new String[][] {{"Tool", "name"}, {"Tool", "shelf"}, {"Member", "fullName"}, {"Member", "email"}}) {
            Map<String, Object> s = byName(list(doc.get("types")), sf[0]);
            Object t = byName(list(s.get("fields")), sf[1]).get("t");
            check(same(t, ptrString), sf[0] + "." + sf[1] + " is " + Json.write(t).replaceAll("\\s+", " "));
            Map<String, Object> ctor = funcByKey(doc, "New" + sf[0]);
            Map<String, Object> lit = map(map(list(map(list(ctor.get("body")).get(0)).get("rhs")).get(0)).get("x"));
            check(byNameOrNull(list(lit.get("fields")), sf[1]) == null, "New" + sf[0] + " sets " + sf[1] + ": its zero value is not nil");
        }
    }

    private void lendingGormCallsAreForeign() throws Exception {
        Map<String, Object> doc = lending();
        String catalog = "grails-app/services/lending/CatalogService.groovy";
        foreignWith(doc, catalog + ":" + lineOf(lendingDir().resolve(catalog), "Tool.findByName(n)"), "gorm: findByName");
        foreignWith(doc, LOAN_SERVICE + ":" + lineOf(lendingDir().resolve(LOAN_SERVICE), "loan.save()"), "gorm: save");
    }

    private void foreignWith(Map<String, Object> doc, String pos, String prefix) {
        boolean refusal = false;
        for (Object r : list(doc.get("refusals"))) {
            if (pos.equals(map(r).get("pos")) && ((String) map(r).get("reason")).startsWith(prefix)) {
                refusal = true;
            }
        }
        check(refusal, "no refusal starting " + prefix + " at " + pos);
        boolean foreign = false;
        for (Map<String, Object> f : foreigns(doc.get("funcs"))) {
            if (pos.equals(f.get("pos")) && ((String) f.get("reason")).startsWith(prefix)) {
                foreign = true;
            }
        }
        check(foreign, "no foreign statement starting " + prefix + " at " + pos);
    }

    private void lendingSpringInjection() throws Exception {
        Map<String, Object> doc = lending();
        Path src = lendingDir().resolve(LOAN_SERVICE);
        for (String field : List.of("def catalogService", "FeeService feeService")) {
            String pos = LOAN_SERVICE + ":" + lineOf(src, field);
            check(reasonsAt(doc, pos).equals(List.of("spring: injected dependency")), pos + " " + reasonsAt(doc, pos));
        }
        foreignWith(doc, LOAN_SERVICE + ":" + lineOf(src, "catalogService.toolById"), "spring: call through an injected dependency");
        foreignWith(doc, LOAN_SERVICE + ":" + lineOf(src, "feeService.lateFeeCents"), "spring: call through an injected dependency");
        Map<String, Object> s = byName(list(doc.get("types")), "LoanService");
        check(list(s.get("fields")).isEmpty(), "LoanService holds the injected fields: " + Json.write(s.get("fields")));
    }

    private void lendingDomainMetadata() throws Exception {
        Map<String, Object> doc = lending();
        String[][] want = {
            {"grails-app/domain/lending/Loan.groovy", "static belongsTo", "belongsTo"},
            {"grails-app/domain/lending/Loan.groovy", "static mapping", "mapping"},
            {"grails-app/domain/lending/Member.groovy", "static hasMany", "hasMany"},
            {"grails-app/domain/lending/Member.groovy", "static constraints", "constraints"},
            {"grails-app/domain/lending/Tool.groovy", "static hasMany", "hasMany"},
            {"grails-app/domain/lending/Tool.groovy", "static constraints", "constraints"},
            {"grails-app/domain/lending/Tool.groovy", "static mapping", "mapping"},
        };
        for (String[] w : want) {
            String pos = w[0] + ":" + lineOf(lendingDir().resolve(w[0]), w[1]);
            check(reasonsAt(doc, pos).equals(List.of("gorm: mapping metadata " + w[2])), pos + " " + reasonsAt(doc, pos));
        }
    }

    private static List<String> reasonsAt(Map<String, Object> doc, String pos) {
        List<String> out = new ArrayList<>();
        for (Object r : list(doc.get("refusals"))) {
            if (pos.equals(map(r).get("pos"))) {
                out.add((String) map(r).get("reason"));
            }
        }
        return out;
    }

    private void lendingTransactionalUnresolvedWithoutJars() throws Exception {
        Map<String, Object> doc = lending();
        String pos = AUDIT + ":" + lineOf(lendingDir().resolve(AUDIT), "import grails.transaction.Transactional");
        check(reasonsAt(doc, pos).equals(List.of("unresolved: grails.transaction.Transactional")), pos + " " + reasonsAt(doc, pos));
        for (Object r : list(doc.get("refusals"))) {
            String p = (String) map(r).get("pos");
            check(!p.startsWith(AUDIT) || p.equals(pos), "a second refusal in the refused artefact: " + p);
        }
        check(byNameOrNull(list(doc.get("types")), "TransactionalAuditService") == null, "TransactionalAuditService was read");
        for (String key : List.of("CatalogService.doubled", "FeeService.receipt", "Member.greeting")) {
            check(funcByKeyOrNull(doc, key) != null, key + " was not read beside the refused artefact");
        }
    }

    private void lendingEveryReasonHasACategory() throws Exception {
        Map<String, Object> doc = lending();
        List<String> bad = new ArrayList<>();
        List<String> reasons = new ArrayList<>(foreignReasons(doc.get("funcs")));
        for (Object r : list(doc.get("refusals"))) {
            reasons.add((String) map(r).get("reason"));
        }
        for (String reason : reasons) {
            int colon = reason.indexOf(": ");
            if (colon < 0 || !CATEGORIES.contains(reason.substring(0, colon))) {
                bad.add(reason);
            }
        }
        check(bad.isEmpty(), "reasons without a category: " + bad);
    }

    private void plainDirectoryReadsAsGroovy() throws Exception {
        Map<String, Object> doc = Main.read(testdata.resolve("typed"), List.of());
        check(!doc.containsKey("refusals"), "refusals: " + Json.write(doc.getOrDefault("refusals", List.of())));
        check(Json.write(doc).equals(Json.write(typed())), "Main.read of typed differs from the plain read");
    }

    private void duplicateSimpleNamesAreRefused() throws Exception {
        Path app = work.resolve("duplicate-app");
        write(app.resolve("application.properties"), "app.grails.version=2.5.6\napp.name=dup\n");
        write(app.resolve("grails-app/domain/a/Book.groovy"), "package a\n\nclass Book {\n    int pages = 0\n}\n");
        write(app.resolve("grails-app/domain/b/Book.groovy"), "package b\n\nclass Book {\n    int copies = 0\n}\n");
        write(app.resolve("grails-app/services/a/ShelfService.groovy"),
            "package a\n\nclass ShelfService {\n    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        check(reasonsAt(doc, "grails-app/domain/a/Book.groovy:3").equals(List.of("unsupported: duplicate simple name Book")),
            "a/Book " + reasonsAt(doc, "grails-app/domain/a/Book.groovy:3"));
        check(reasonsAt(doc, "grails-app/domain/b/Book.groovy:3").equals(List.of("unsupported: duplicate simple name Book")),
            "b/Book " + reasonsAt(doc, "grails-app/domain/b/Book.groovy:3"));
        check(byNameOrNull(list(doc.get("types")), "Book") == null, "a Book was read");
        check(funcByKeyOrNull(doc, "ShelfService.one") != null, "ShelfService.one was not read");
        check("dup".equals(doc.get("name")), "module name " + doc.get("name"));
    }

    private static void write(Path p, String text) throws IOException {
        Files.createDirectories(p.getParent());
        Files.writeString(p, text, StandardCharsets.UTF_8);
    }

    private void theReadmeNamesTheGrailsJars() throws Exception {
        String readme = Files.readString(root.resolve("adapters/groovy/README.md"));
        for (String j : GRAILS_JARS) {
            check(readme.contains("$GRAILS_HOME/" + j), "README.md does not name $GRAILS_HOME/" + j);
        }
        check(readme.contains("CODESPEC_GROOVY_CLASSPATH"), "README.md does not name CODESPEC_GROOVY_CLASSPATH");
        check(!readme.contains("/home/"), "README.md names a home directory");
    }

    private void lendingTransactionalWithJars() throws Exception {
        List<Path> jars = grailsJars();
        Map<String, Object> doc = Main.read(lendingDir(), jars);
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            check(!((String) rm.get("reason")).startsWith("unresolved"), "with the Grails jars: " + rm.get("pos") + " " + rm.get("reason"));
        }
        check(byNameOrNull(list(doc.get("types")), "TransactionalAuditService") != null, "TransactionalAuditService was not read");
        List<String> lines = Files.readAllLines(lendingDir().resolve(AUDIT));
        for (int i = 0; i < lines.size(); i++) {
            String method = Lending.methodName(lines.get(i));
            if (method == null) {
                continue;
            }
            String pos = AUDIT + ":" + (i + 1);
            Map<String, Object> fn = funcByKeyOrNull(doc, "TransactionalAuditService." + method);
            List<String> reasons = reasonsAt(doc, pos);
            boolean refused = fn == null && reasons.size() == 1 && reasons.get(0).startsWith("unsupported: grails transform");
            boolean readFromSource = fn != null && pos.equals(fn.get("pos")) && reasons.isEmpty() && withinMethod(fn, lines, i + 1);
            check(refused || readFromSource, method + " at " + pos + " is neither read from its source body nor refused as"
                + " grails transform: " + (fn == null ? "no function" : Json.write(fn).replaceAll("\\s+", " ")) + " " + reasons);
        }
        List<String> others = new ArrayList<>();
        for (String p : Lending.compare(doc, lendingDir())) {
            if (!p.startsWith(AUDIT)) {
                others.add(p);
            }
        }
        check(others.isEmpty(), "with the Grails jars the other artefacts differ:\n  " + String.join("\n  ", others));
    }

    /** Whether every statement of a function lies in the source lines of the method declared at {@code line}. */
    private static boolean withinMethod(Map<String, Object> fn, List<String> lines, int line) {
        int indent = lines.get(line - 1).length() - lines.get(line - 1).stripLeading().length();
        int end = line;
        while (end < lines.size()) {
            String l = lines.get(end);
            end++;
            if (l.stripLeading().startsWith("}") && l.length() - l.stripLeading().length() == indent) {
                break;
            }
        }
        for (Map<String, Object> s : statements(fn.get("body"))) {
            int at = Lending.lineOf((String) s.get("pos"));
            if (at <= line || at > end) {
                return false;
            }
        }
        return true;
    }

    private void lendingNoTransformIdentifier() throws Exception {
        List<Path> jars = grailsJars();
        Map<String, Object> doc = Main.read(lendingDir(), jars);
        List<String> found = new ArrayList<>();
        holds(doc, "(module)", found);
        check(found.isEmpty(), "identifiers of the transactional transform in the read: " + found);
    }

    /** Every identifier of {@link #TRANSFORM_IDENTIFIERS} in a string under {@code o}, with the nearest enclosing position. */
    private static void holds(Object o, String pos, List<String> found) {
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            String here = m.get("pos") instanceof String ? (String) m.get("pos") : pos;
            for (Map.Entry<String, Object> e : m.entrySet()) {
                holds(e.getKey(), here, found);
                holds(e.getValue(), here, found);
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                holds(v, pos, found);
            }
        } else if (o instanceof String) {
            for (String id : TRANSFORM_IDENTIFIERS) {
                if (((String) o).contains(id)) {
                    found.add(id + " at " + pos);
                }
            }
        }
    }

    // ---- adversary: story:grails-artefacts ----

    /** A Grails 2.5.6 application named {@code library} under the work dir, emptied first. */
    private Path adversaryApp(String name) throws IOException {
        Path app = work.resolve("adversary-grails/" + name);
        if (Files.exists(app)) {
            try (Stream<Path> s = Files.walk(app)) {
                for (Path p : s.sorted(java.util.Comparator.reverseOrder()).toList()) {
                    Files.delete(p);
                }
            }
        }
        write(app.resolve("application.properties"), "app.grails.version=2.5.6\napp.name=library\n");
        return app;
    }

    /**
     * Grails autowires every service by name (ServicesGrailsPlugin: {@code bean.autowire = true},
     * {@code innerBean.autowire = "byName"}), and {@code grailsApplication} is always a bean. {@code
     * def grailsApplication} is the idiomatic Grails 2 way to reach the configuration; the reader
     * refuses it as {@code dynamic} and with it the whole service, so {@code one} is lost.
     */
    private void adversaryGrailsApplicationInjected() throws Exception {
        Path app = adversaryApp("grails-application");
        String svc = "grails-app/services/lib/AppService.groovy";
        write(app.resolve(svc), "package lib\n\nclass AppService {\n\n    def grailsApplication\n\n"
            + "    int one() {\n        return 1\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        check(funcByKeyOrNull(doc, "AppService.one") != null, "AppService.one is not read; refusals: " + reasonsAt(doc, svc + ":5"));
        check(reasonsAt(doc, svc + ":5").equals(List.of("spring: injected dependency")), "def grailsApplication at " + svc
            + ":5 is " + reasonsAt(doc, svc + ":5"));
    }

    /**
     * story:grails-artefacts wires an uninitialised domain property to the shared ptr(T) rule of
     * story:nullable-reference-fields; the Java reader reads a write of such a field and its
     * comparison with null with no refusal. The Grails layer refuses every read and write of it.
     */
    private void adversaryNullablePropertyUses() throws Exception {
        Path app = adversaryApp("nullable-uses");
        write(app.resolve("grails-app/domain/lib/Book.groovy"), "package lib\n\nclass Book {\n\n    String title\n"
            + "    int pages = 0\n\n"
            + "    void rename(String t) {\n        title = t\n    }\n\n"
            + "    boolean untitled() {\n        return title == null\n    }\n\n"
            + "    String titleOr(String fallback) {\n        if (title == null) {\n            return fallback\n        }\n"
            + "        return title\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        // Coordinator decision (story:grails-artefacts, round 3): the shared ptr(T) rule holds, so
        // titleOr is refused exactly at its `return title`, as the Java reader refuses it; the
        // other two methods read with no refusal.
        String book = "grails-app/domain/lib/Book.groovy";
        int ret = Files.readAllLines(app.resolve(book)).stream().map(String::trim).toList().indexOf("return title") + 1;
        Map<String, Object> titleOr = funcByKeyOrNull(doc, "Book.titleOr");
        check(titleOr != null, "Book.titleOr is not read as a function");
        List<String> at = new ArrayList<>();
        for (Map<String, Object> f : foreigns(titleOr)) {
            at.add(f.get("pos") + " " + f.get("reason"));
        }
        check(at.size() == 1 && at.get(0).startsWith(book + ":" + ret + " unsupported: property title, null until set, returned"),
            "Book.titleOr is not refused exactly at its return title (" + book + ":" + ret + "): " + at);
        check(reasonsAt(doc, book + ":" + ret).size() == 1, "refusals at " + book + ":" + ret + ": " + reasonsAt(doc, book + ":" + ret));
        List<String> refused = new ArrayList<>();
        for (String key : List.of("Book.rename", "Book.untitled")) {
            Map<String, Object> fn = funcByKeyOrNull(doc, key);
            if (fn == null) {
                refused.add(key + ": not read");
            } else if (!foreigns(fn).isEmpty()) {
                refused.add(key + ": " + foreignReasons(fn));
            }
        }
        check(refused.isEmpty(), "uses of the ptr(string) property title refused: " + refused);
    }

    /**
     * story:grails-artefacts, Context: "All of grails-app/ and src/ (src/groovy, src/java) go to
     * the compiler, so a service that references a class from ... src/ resolves." A src/java class
     * named in a service signature refuses the whole service as unresolved.
     */
    private void adversarySrcJavaResolves() throws Exception {
        Path app = adversaryApp("src-java");
        write(app.resolve("src/java/lib/JavaMath.java"), "package lib;\n\npublic final class JavaMath {\n"
            + "    private JavaMath() {}\n\n    public static int twice(int n) {\n        return n * 2;\n    }\n}\n");
        String svc = "grails-app/services/lib/JavaUserService.groovy";
        write(app.resolve(svc), "package lib\n\nclass JavaUserService {\n\n"
            + "    int viaSrcJava(JavaMath m, int n) {\n        return n\n    }\n\n"
            + "    int pure(int n) {\n        return n + 2\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        for (Object r : list(doc.get("refusals"))) {
            check(!((String) map(r).get("reason")).startsWith("unresolved"), map(r).get("pos") + " " + map(r).get("reason"));
        }
        check(funcByKeyOrNull(doc, "JavaUserService.pure") != null, "JavaUserService.pure is not read");
    }

    /**
     * Grails 2 lets a class in src/groovy be a GORM entity through {@code @grails.persistence.Entity}.
     * With the six jars README.md names, its local transformation needs spring-context, and the
     * NoClassDefFoundError fails the whole read, though src/groovy is resolve-only and "yields no
     * struct, function or refusal". The same application without the jars reads.
     */
    private void adversaryEntityInSrcGroovy() throws Exception {
        List<Path> jars = grailsJars();
        Path app = adversaryApp("entity-src-groovy");
        write(app.resolve("src/groovy/lib/Record.groovy"), "package lib\n\nimport grails.persistence.Entity\n\n@Entity\n"
            + "class Record {\n\n    int pages = 0\n}\n");
        write(app.resolve("grails-app/domain/lib/Shelf.groovy"), "package lib\n\nclass Shelf {\n\n    int slots = 0\n\n"
            + "    int more(int n) {\n        return slots + n\n    }\n}\n");
        Map<String, Object> doc;
        try {
            doc = Main.read(app, jars);
        } catch (Throwable e) {
            throw new AssertionError("the read failed: " + e);
        }
        check(funcByKeyOrNull(doc, "Shelf.more") != null, "Shelf.more is not read");
    }

    /**
     * Spring's autowire by name sets a service property whatever its initializer, so in Grails
     * {@code countService} is the shared singleton, not {@code new CountService()}. Run with
     * spring-beans 4.1.9 from the Grails 2.5.6 distribution: twice() gives 3 after one earlier bump
     * of the bean, 2 on a fresh instance. The reader reads twice() with no refusal on its own instance.
     */
    private void adversaryInitialisedServiceField() throws Exception {
        Path app = adversaryApp("initialised-service-field");
        write(app.resolve("grails-app/services/lib/CountService.groovy"), "package lib\n\nclass CountService {\n\n"
            + "    int calls = 0\n\n    int bump() {\n        calls = calls + 1\n        return calls\n    }\n}\n");
        String svc = "grails-app/services/lib/UserService.groovy";
        write(app.resolve(svc), "package lib\n\nclass UserService {\n\n    CountService countService = new CountService()\n\n"
            + "    int twice() {\n        countService.bump()\n        return countService.bump()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        Map<String, Object> fn = funcByKeyOrNull(doc, "UserService.twice");
        check(fn == null || !foreigns(fn).isEmpty() || !reasonsAt(doc, svc + ":5").isEmpty(),
            "UserService.twice is read as calling bump on the service's own CountService, with no refusal");
    }

    /**
     * A method-level {@code @Transactional} on {@code size(int)} rewrites only that method. Its
     * overload {@code size(String)} is matched by name and parameter count, emptied and refused as
     * rewritten, which it is not; its own reason, overloading, is lost.
     */
    private void adversaryOverloadNotRewritten() throws Exception {
        List<Path> jars = grailsJars();
        Path app = adversaryApp("overload-transform");
        String svc = "grails-app/services/lib/OverloadTxService.groovy";
        write(app.resolve(svc), "package lib\n\nimport grails.transaction.Transactional\n\nclass OverloadTxService {\n\n"
            + "    @Transactional\n    int size(int n) {\n        return n\n    }\n\n"
            + "    int size(String s) {\n        return s.length()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, jars);
        int line = lineOf(app.resolve(svc), "int size(String s)");
        for (String reason : reasonsAt(doc, svc + ":" + line)) {
            check(!reason.startsWith("unsupported: grails transform"), svc + ":" + line + " " + reason);
        }
    }

    /** {@code listOrderBy*} is a GORM dynamic method (Grails 2.5 reference, Domain Classes). */
    private void adversaryListOrderBy() throws Exception {
        Path app = adversaryApp("list-order-by");
        write(app.resolve("grails-app/domain/lib/Book.groovy"), "package lib\n\nclass Book {\n\n    int pages = 0\n}\n");
        String svc = "grails-app/services/lib/ShelfService.groovy";
        write(app.resolve(svc), "package lib\n\nclass ShelfService {\n\n"
            + "    List<Book> ordered() {\n        return Book.listOrderByPages()\n    }\n}\n");
        Map<String, Object> doc = Main.read(app, List.of());
        foreignWith(doc, svc + ":6", "gorm: listOrderByPages");
    }

    // ---- adversary: reads whose meaning differs from Groovy's ----

    /** {@code <file>:<line>} of every line marked {@code // diverges} in a fixture under testdata/adversary. */
    private List<String> divergeMarkers(String fixture) throws IOException {
        List<String> found = new ArrayList<>();
        try (Stream<Path> files = Files.list(testdata.resolve("adversary/" + fixture))) {
            for (Path f : files.sorted().toList()) {
                List<String> lines = Files.readAllLines(f);
                for (int i = 0; i < lines.size(); i++) {
                    if (lines.get(i).contains("// diverges")) {
                        found.add(f.getFileName() + ":" + (i + 1));
                    }
                }
            }
        }
        check(!found.isEmpty(), "no // diverges marker in " + fixture);
        return found;
    }

    /**
     * Passes when the reader refuses at a marked line, or when no statement read at a marked line
     * (outside foreign) is {@code bad}: the shape whose meaning differs from what Groovy does.
     */
    private void notReadAs(String fixture, java.util.function.Predicate<Map<String, Object>> bad) throws Exception {
        Map<String, Object> doc = Reader.read(testdata.resolve("adversary/" + fixture));
        List<String> marks = divergeMarkers(fixture);
        for (Object r : list(doc.get("refusals"))) {
            if (marks.contains(map(r).get("pos"))) {
                return;
            }
        }
        List<String> wrong = new ArrayList<>();
        for (Map<String, Object> s : statements(doc.get("funcs"))) {
            if (marks.contains(s.get("pos")) && !"foreign".equals(s.get("k")) && bad.test(s)) {
                wrong.add(s.get("pos") + " " + Json.write(s).replaceAll("\\s+", " "));
            }
        }
        check(wrong.isEmpty(), "read with a meaning Groovy does not give it, no refusal at " + marks + ": " + wrong);
    }

    /** Every object under {@code o} carrying a {@code pos}: functions and statements. */
    private static List<Map<String, Object>> statements(Object o) {
        List<Map<String, Object>> out = new ArrayList<>();
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if (m.containsKey("pos") && m.containsKey("k")) {
                out.add(m);
            }
            for (Object v : m.values()) {
                out.addAll(statements(v));
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                out.addAll(statements(v));
            }
        }
        return out;
    }

    /** An assignment, ++ or -- whose target is the field {@code copies} itself. */
    private static boolean writesFieldCopies(Map<String, Object> s) {
        Object k = s.get("k");
        if (!"assign".equals(k) && !"incr".equals(k) && !"decr".equals(k)) {
            return false;
        }
        Object target = "assign".equals(k) ? list(s.get("lhs")).get(0) : s.get("x");
        Map<String, Object> t = map(target);
        return "field".equals(t.get("k")) && "copies".equals(t.get("name"));
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

    /** Positions of every groovy foreign statement under {@code o}. */
    private static List<String> foreignPositions(Object o) {
        List<String> out = new ArrayList<>();
        for (Map<String, Object> f : foreigns(o)) {
            out.add((String) f.get("pos"));
        }
        return out;
    }

    private static List<String> foreignReasons(Object o) {
        List<String> out = new ArrayList<>();
        for (Map<String, Object> f : foreigns(o)) {
            out.add((String) f.get("reason"));
        }
        return out;
    }

    private static List<Map<String, Object>> foreigns(Object o) {
        List<Map<String, Object>> out = new ArrayList<>();
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if ("foreign".equals(m.get("k")) && "groovy".equals(m.get("lang"))) {
                out.add(m);
            }
            for (Object v : m.values()) {
                out.addAll(foreigns(v));
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                out.addAll(foreigns(v));
            }
        }
        return out;
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

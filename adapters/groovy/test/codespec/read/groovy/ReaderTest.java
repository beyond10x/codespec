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
 * It prints one line per case and a summary {@code N passed, M failed}, and exits 1 when any case
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
        String only = args.length == 4 ? args[3] : null;
        if (only != null) {
            cases.keySet().removeIf(k -> !k.contains(only));
        }
        int passed = 0;
        int failed = 0;
        for (Map.Entry<String, Case> c : cases.entrySet()) {
            try {
                c.getValue().run();
                System.out.println("ok   " + c.getKey());
                passed++;
            } catch (Throwable e) {
                System.out.println("FAIL " + c.getKey() + ": " + e);
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed");
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

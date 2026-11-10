package codespec.read;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * Second adversary pass on the Java reader's project forms (Panache field access, Panache static
 * methods, calls of another package, persistence arguments), run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.read.Adversary3Test &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt; &lt;adapters dir&gt;</pre>
 *
 * Needs {@code CODESPEC_JAVA_CLASSPATH} as {@code task java-quarkus-prepare} writes it. Prints one
 * line per case and {@code N passed, M failed}; exits 1 when any case fails.
 *
 * <p>The Java behaviour the cases assert is Quarkus 3.27's: its own {@code
 * PanacheHibernateCommonResourceProcessor} rewrites every access of a public field of a managed class
 * ({@code @Entity}, {@code @MappedSuperclass}, {@code @Embeddable}, Panache or not) in every class that
 * uses it, the entity itself included, into the field's getter, except inside the getter itself and in
 * a constructor of the entity's hierarchy.
 */
public final class Adversary3Test {
    private static final String CLASSPATH_ENV = "CODESPEC_JAVA_CLASSPATH";

    private final Path testdata;
    private final java.util.HashMap<String, Map<String, Map<String, Object>>> read = new java.util.HashMap<>();

    private Adversary3Test(Path root) {
        this.testdata = root.resolve("adapters/java/testdata/adversary3");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 4) {
            System.err.println("usage: Adversary3Test <repo root> <codespec binary> <work dir> <adapters dir>");
            System.exit(2);
        }
        Adversary3Test t = new Adversary3Test(Path.of(args[0]));
        Files.createDirectories(Path.of(args[2]));
        List<String> names = new ArrayList<>();
        List<Case> cases = new ArrayList<>();
        names.add("adversary3_a_public_field_of_a_plain_jpa_entity_with_a_declared_getter_read_from_outside_is_refused");
        cases.add(() -> t.refusedAt("project-access", "farm", "Use.java", "return c.yield;"));
        names.add("adversary3_a_public_field_of_a_panache_entity_read_in_its_own_method_where_it_declares_the_getter_is_refused");
        cases.add(() -> t.refusedAt("project-access", "farm", "Fruit.java", "return weight + by;"));
        names.add("adversary3_a_panache_field_whose_entity_declares_only_a_same_named_method_with_a_parameter_is_read");
        cases.add(t::aSameNamedMethodWithAParameterIsNoGetter);
        names.add("adversary3_an_unqualified_panache_static_call_in_a_class_nested_in_the_entity_is_refused");
        cases.add(() -> t.refusedAt("project-statics", "shop", "Fruit.java", "return deleteAll();"));
        names.add("adversary3_a_statically_imported_panache_method_is_called_on_the_entity_it_was_imported_from");
        cases.add(t::aStaticallyImportedPanacheMethodNamesItsEntity);
        names.add("adversary3_a_call_of_another_packages_method_that_throws_its_checked_exception_keeps_the_error");
        cases.add(t::aCallThatThrowsAnotherPackagesExceptionKeepsTheError);
        names.add("adversary3_a_call_of_another_packages_optional_method_is_declared_as_that_module_declares_it");
        cases.add(t::aCallOfAnotherPackagesOptionalMethodMatchesItsModule);
        names.add("adversary3_a_line_comment_inside_a_persistence_argument_does_not_swallow_its_value");
        cases.add(t::aLineCommentInsideAnArgumentDoesNotSwallowTheValue);
        names.add("adversary3_a_quote_in_a_comment_inside_a_persistence_argument_keeps_the_literal_as_written");
        cases.add(t::aQuoteInACommentKeepsTheLiteral);
        names.add("adversary3_each_project_reads_twice_byte_equal");
        cases.add(t::eachProjectReadsTwiceByteEqual);
        // Green on c35dad2; each fails on a mutant of a changed line the other suites miss.
        names.add("adversary3_a_field_of_a_struct_of_this_package_reached_through_a_subclass_of_another_package_is_refused");
        cases.add(() -> t.refusedAt("project-throws", "b", "Use.java", "return s.v;"));
        names.add("adversary3_a_call_of_an_overload_another_package_refuses_is_refused");
        cases.add(() -> t.refusedAt("project-throws", "b", "Use.java", "return Two.twice(\"x\");"));
        names.add("adversary3_a_call_of_a_method_of_an_interface_another_package_refuses_is_refused");
        cases.add(() -> t.refusedAt("project-throws", "b", "Use.java", "return s.area();"));
        names.add("adversary3_a_nested_class_of_another_package_its_module_does_not_declare_keeps_its_qualified_name");
        cases.add(t::aNestedClassItsModuleDoesNotDeclareKeepsItsQualifiedName);
        names.add("adversary3_an_unqualified_panache_static_call_in_the_entity_carries_the_entity");
        cases.add(t::anUnqualifiedPanacheCallInTheEntityCarriesTheEntity);
        names.add("adversary3_a_persistence_argument_holding_a_text_block_is_refused");
        cases.add(() -> t.refusedAt("project-marks", "marks", "Row.java", "@Column(columnDefinition = \"\"\""));
        // The reader takes the refusal branch of the two cases above that allow either.
        names.add("adversary3_a_call_of_another_packages_method_that_throws_is_refused_at_its_line");
        cases.add(() -> t.refusedAt("project-throws", "b", "Use.java", "long a = Lib.load(k);"));
        names.add("adversary3_a_call_of_another_packages_optional_method_is_refused_at_its_line");
        cases.add(() -> t.refusedAt("project-throws", "b", "Use.java", "return Lib.find(k).isPresent();"));
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

    // ---- cases ----

    /**
     * {@code Plum} declares {@code getSize(long)}, not a getter. Quarkus generates {@code getSize()},
     * which reads the field, and rewrites {@code p.size} into it: the value is the field's. A refusal
     * here is a refusal of a read that means what the Java means.
     */
    private void aSameNamedMethodWithAParameterIsNoGetter() throws Exception {
        Map<String, Object> farm = module("project-access", "farm");
        int line = lineOf(src("project-access", "Use.java"), "return p.size;");
        for (Object r : list(farm.get("refusals"))) {
            check(!("Use.java:" + line).equals(map(r).get("pos")), "p.size refused: " + compact(r));
        }
    }

    /**
     * {@code import static shop.Fruit.deleteAll;} then {@code deleteAll()} in {@code Use}: javac calls
     * {@code Fruit.deleteAll()}, which deletes the rows of {@code Fruit}. The call is refused or carries
     * {@code Fruit} as its entity.
     */
    private void aStaticallyImportedPanacheMethodNamesItsEntity() throws Exception {
        Map<String, Object> shop = module("project-statics", "shop");
        int line = lineOf(src("project-statics", "Use.java"), "return deleteAll();");
        if (refused(shop, "Use.java", line)) {
            return;
        }
        String body = compact(funcByKey(shop, "imported").get("body"));
        check(body.contains("\"targs\":[{\"k\":\"named\",\"name\":\"Fruit\"}]"), "imported reads " + body);
    }

    /**
     * {@code b.lib.Lib.load} throws {@code b.lib.LibError}, a checked exception {@code b.lib} reads as
     * its error: its module declares {@code load} with an {@code error} result. {@code b.Use.total}
     * calls it and Java propagates the exception ({@code total(-1)} throws {@code Negative}). The call
     * is refused, or its extern keeps the error result.
     */
    private void aCallThatThrowsAnotherPackagesExceptionKeepsTheError() throws Exception {
        Map<String, Object> b = module("project-throws", "b");
        Map<String, Object> lib = module("project-throws", "b.lib");
        int line = lineOf(src("project-throws", "Use.java"), "long a = Lib.load(k);");
        if (refused(b, "Use.java", line)) {
            return;
        }
        List<Object> declared = list(funcByKey(lib, "load").get("results"));
        List<Object> extern = list(externNamed(b, "b.lib", "load").get("results"));
        check(declared.size() == extern.size(), "b.lib declares load with results " + compact(declared)
            + ", b calls it as an extern with results " + compact(extern) + " and no refusal");
    }

    /**
     * {@code b.lib.Lib.find} returns {@code Optional<Long>}; {@code b.lib} declares it with the results
     * {@code (int64, bool)}. {@code b} calls it as an extern returning a jar {@code Optional} and calls
     * {@code isPresent} on that object, which the function {@code b.lib} declares does not return.
     */
    private void aCallOfAnotherPackagesOptionalMethodMatchesItsModule() throws Exception {
        Map<String, Object> b = module("project-throws", "b");
        Map<String, Object> lib = module("project-throws", "b.lib");
        int line = lineOf(src("project-throws", "Use.java"), "return Lib.find(k).isPresent();");
        if (refused(b, "Use.java", line)) {
            return;
        }
        List<Object> declared = list(funcByKey(lib, "find").get("results"));
        List<Object> extern = list(externNamed(b, "b.lib", "find").get("results"));
        check(compact(declared).equals(compact(extern)), "b.lib declares find with results " + compact(declared)
            + ", b calls it as an extern with results " + compact(extern) + " and no refusal");
    }

    /**
     * {@code @Column(name = // the legacy column\n "row_label")}: the column is {@code row_label}. On one
     * line the value follows a line comment, so the recorded argument no longer names it.
     */
    private void aLineCommentInsideAnArgumentDoesNotSwallowTheValue() throws Exception {
        Map<String, Object> marks = module("project-marks", "marks");
        int line = lineOf(src("project-marks", "Row.java"), "@Column(name = // the legacy column");
        if (refused(marks, "Row.java", line)) {
            return;
        }
        for (Object m : list(marks.get("persistence"))) {
            if ("Row.label".equals(map(m).get("at"))) {
                for (Object a : list(map(m).get("args"))) {
                    check(!((String) a).contains("//"), "Row.label argument " + a + ": its value is inside a line comment");
                }
                return;
            }
        }
        throw new AssertionError("Row.label has neither a refusal nor a Column mark");
    }

    /** {@code @Column(name = /* " *}{@code / "two  spaces")}: the literal holds two spaces. */
    private void aQuoteInACommentKeepsTheLiteral() throws Exception {
        Map<String, Object> marks = module("project-marks", "marks");
        int line = lineOf(src("project-marks", "Row.java"), "@Column(name = /* \" */");
        if (refused(marks, "Row.java", line)) {
            return;
        }
        for (Object m : list(marks.get("persistence"))) {
            if ("Row.spaced".equals(map(m).get("at"))) {
                List<Object> args = list(map(m).get("args"));
                check(args.size() == 1 && ((String) args.get(0)).contains("\"two  spaces\""), "Row.spaced arguments " + args
                    + ": the literal lost a space");
                return;
            }
        }
        throw new AssertionError("Row.spaced has neither a refusal nor a Column mark");
    }

    /**
     * {@code Outer.In} of {@code b.lib} shares its simple name with the top-level {@code b.lib.In};
     * {@code b.lib} refuses the nested one, so a type of {@code b} naming it must not name {@code In}.
     */
    private void aNestedClassItsModuleDoesNotDeclareKeepsItsQualifiedName() throws Exception {
        Map<String, Object> param = map(list(funcByKey(module("project-throws", "b"), "inner").get("params")).get(0));
        check("Outer.In".equals(map(param.get("t")).get("name")), "inner takes " + compact(param.get("t")));
    }

    /** {@code count()} in a static method of {@code Fruit}: javac calls {@code Fruit.count()}. */
    private void anUnqualifiedPanacheCallInTheEntityCarriesTheEntity() throws Exception {
        String body = compact(funcByKey(module("project-statics", "shop"), "all").get("body"));
        check(body.contains("\"name\":\"count\",\"targs\":[{\"k\":\"named\",\"name\":\"Fruit\"}]"), "all reads " + body);
    }

    /** Two reads of the same project give the same bytes. */
    private void eachProjectReadsTwiceByteEqual() throws Exception {
        for (String p : List.of("project-access", "project-statics", "project-throws", "project-marks")) {
            String one = Json.write(Reader.readProject(testdata.resolve(p), classpath()));
            String two = Json.write(Reader.readProject(testdata.resolve(p), classpath()));
            check(one.equals(two), p + " reads differently twice");
        }
    }

    // ---- helpers ----

    private Map<String, Object> module(String project, String pkg) throws Exception {
        if (!read.containsKey(project)) {
            read.put(project, Reader.readProject(testdata.resolve(project), classpath()));
        }
        Map<String, Object> m = read.get(project).get(pkg);
        check(m != null, project + " has no package " + pkg + ": " + read.get(project).keySet());
        return m;
    }

    private Path src(String project, String file) throws Exception {
        try (var files = Files.walk(testdata.resolve(project).resolve("src/main/java"))) {
            return files.filter(f -> f.getFileName().toString().equals(file)).findFirst().orElseThrow();
        }
    }

    private void refusedAt(String project, String pkg, String file, String text) throws Exception {
        Map<String, Object> doc = module(project, pkg);
        int line = lineOf(src(project, file), text);
        check(refused(doc, file, line), "no refusal at " + file + ":" + line + " (" + text + "); refusals "
            + compact(doc.get("refusals")));
    }

    private static boolean refused(Map<String, Object> doc, String file, int line) {
        for (Object r : list(doc.get("refusals"))) {
            if ((file + ":" + line).equals(map(r).get("pos"))) {
                return true;
            }
        }
        return false;
    }

    private static Map<String, Object> externNamed(Map<String, Object> doc, String path, String name) {
        for (Object x : list(doc.get("externs"))) {
            if (path.equals(map(x).get("path")) && name.equals(map(x).get("name"))) {
                return map(x);
            }
        }
        throw new AssertionError("no extern " + path + "." + name + " in " + compact(doc.get("externs")));
    }

    private static Map<String, Object> funcByKey(Map<String, Object> doc, String key) {
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
        throw new AssertionError("no func " + key);
    }

    private static String compact(Object o) {
        return o == null ? "[]" : Json.write(o).replaceAll("\\s+", "");
    }

    private static List<Path> classpath() {
        String cp = System.getenv(CLASSPATH_ENV);
        check(cp != null && !cp.isEmpty(), CLASSPATH_ENV + " is not set: task java-quarkus-prepare writes it");
        return Reader.classpath(cp);
    }

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

    private static int lineOf(Path file, String text) throws java.io.IOException {
        List<String> lines = Files.readAllLines(file);
        for (int i = 0; i < lines.size(); i++) {
            if (lines.get(i).contains(text)) {
                return i + 1;
            }
        }
        throw new AssertionError("no " + text + " in " + file);
    }
}

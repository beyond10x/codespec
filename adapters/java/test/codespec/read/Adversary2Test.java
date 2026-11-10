package codespec.read;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;

/**
 * Adversary cases for the project forms of the Java reader (second constructors, static nested
 * classes, static final fields, fields of another package, Panache entities, persistence marks), run
 * as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.read.Adversary2Test &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt; &lt;adapters dir&gt;</pre>
 *
 * Needs {@code CODESPEC_JAVA_CLASSPATH} as {@code task java-quarkus-prepare} writes it. Prints one
 * line per case and {@code N passed, M failed}; exits 1 when any case fails.
 */
public final class Adversary2Test {
    private static final String CLASSPATH_ENV = "CODESPEC_JAVA_CLASSPATH";

    private final Path codespec;
    private final Path work;
    private final Path testdata;
    private Map<String, Map<String, Object>> panache;
    private Map<String, Map<String, Object>> packages;

    private Adversary2Test(Path root, Path codespec, Path work) {
        this.codespec = codespec;
        this.work = work;
        this.testdata = root.resolve("adapters/java/testdata/adversary2");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 4) {
            System.err.println("usage: Adversary2Test <repo root> <codespec binary> <work dir> <adapters dir>");
            System.exit(2);
        }
        Adversary2Test t = new Adversary2Test(Path.of(args[0]), Path.of(args[1]), Path.of(args[2]));
        Files.createDirectories(t.work);
        List<String> names = new ArrayList<>();
        List<Case> cases = new ArrayList<>();
        names.add("adversary2_a_field_of_a_panache_entity_whose_getter_quarkus_names_getx_coord_is_refused");
        cases.add(() -> t.refusedAt(t.panache(), "project-panache", "Desk.java", "return s.xCoord;", "Panache"));
        names.add("adversary2_a_write_of_a_panache_entity_field_whose_setter_the_entity_declares_is_refused");
        cases.add(() -> t.refusedAt(t.panache(), "project-panache", "Desk.java", "g.level = 5;", "Panache"));
        names.add("adversary2_a_persistence_argument_written_over_two_lines_reads_into_a_module_strict_reading_admits");
        cases.add(t::aPersistenceArgumentOverTwoLinesIsAValidModule);
        names.add("adversary2_c_class_get_name_of_a_nested_class_is_its_binary_name_with_a_dollar");
        cases.add(t::aNestedClassNameIsItsBinaryName);
        names.add("adversary2_a_static_final_field_set_by_a_java_lang_call_is_refused");
        cases.add(() -> t.refusedAt(t.panache(), "project-panache", "Logs.java", "System.currentTimeMillis()", "a static field initialized other than"));
        names.add("adversary2_a_static_final_field_set_by_an_instance_method_of_a_jar_object_is_refused");
        cases.add(() -> t.refusedAt(t.panache(), "project-panache", "Logs.java", ".getName();", "a static field initialized other than"));
        names.add("adversary2_a_field_initializer_refused_is_one_refusal_not_one_per_constructor");
        cases.add(t::aFieldInitializerRefusedIsOneRefusal);
        names.add("adversary2_a_class_whose_one_constructor_is_refused_gets_no_made_up_constructor");
        cases.add(t::aClassWhoseOneConstructorIsRefusedGetsNoMadeUpConstructor);
        names.add("adversary2_a_persistence_annotation_on_a_static_field_stays_a_refusal");
        cases.add(() -> t.refusedAt(t.panache(), "project-panache", "Marks.java", "@Column", "annotation jakarta.persistence.Column"));
        names.add("adversary2_a_persistence_annotation_on_a_method_stays_a_refusal");
        cases.add(() -> t.refusedAt(t.panache(), "project-panache", "Marks.java", "@Id", "annotation jakarta.persistence.Id"));
        names.add("adversary2_an_entity_annotation_on_a_class_read_as_no_struct_is_refused_or_recorded");
        cases.add(t::anEntityAnnotationOnAnExceptionClassIsRefusedOrRecorded);
        names.add("adversary2_an_injection_point_names_as_its_bean_only_a_bean_the_module_lists");
        cases.add(t::anInjectionPointNamesOnlyABeanTheModuleLists);
        names.add("adversary2_an_extern_type_of_another_package_names_a_type_that_package_declares");
        cases.add(t::anExternTypeOfAnotherPackageNamesATypeItDeclares);
        names.add("adversary2_an_extern_call_of_another_package_names_a_function_that_package_declares");
        cases.add(t::anExternCallOfAnotherPackageNamesAFunctionItDeclares);
        names.add("adversary2_a_field_written_through_an_object_of_another_package_is_refused_or_reached_through_a_pointer");
        cases.add(t::aFieldWrittenThroughAnObjectOfAnotherPackageIsRefusedOrAPointer);
        names.add("adversary2_a_call_of_another_packages_second_constructor_names_its_second_function");
        cases.add(t::aCallOfAnotherPackagesSecondConstructorNamesItsSecondFunction);
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

    // ---- fixtures ----

    private Map<String, Object> panache() throws Exception {
        if (panache == null) {
            panache = Reader.readProject(testdata.resolve("project-panache"), classpath());
        }
        check(panache.keySet().equals(Set.of("shop")), "packages " + panache.keySet());
        return panache.get("shop");
    }

    private Map<String, Map<String, Object>> packages() throws Exception {
        if (packages == null) {
            packages = Reader.readProject(testdata.resolve("project-packages"), classpath());
        }
        check(packages.keySet().equals(Set.of("app", "app.lib")), "packages " + packages.keySet());
        return packages;
    }

    // ---- cases ----

    /**
     * {@code @Column(name =\n "body_text")}: a common way to break a long annotation. The read must
     * be a module {@code codespec validate} admits, as every other read of the project is.
     */
    private void aPersistenceArgumentOverTwoLinesIsAValidModule() throws Exception {
        Path doc = work.resolve("panache.shop.json");
        Files.writeString(doc, Json.write(panache()), StandardCharsets.UTF_8);
        Path err = work.resolve("panache.shop.validate.err");
        Process p = new ProcessBuilder(codespec.toString(), "validate", doc.toString())
            .redirectOutput(ProcessBuilder.Redirect.DISCARD).redirectError(err.toFile()).start();
        int code = p.waitFor();
        check(code == 0, "codespec validate exit " + code + ": " + Files.readString(err).strip());
    }

    /** {@code Logger.getLogger(Logs.Part.class.getName())}: Java gives {@code shop.Logs$Part}. */
    private void aNestedClassNameIsItsBinaryName() throws Exception {
        Map<String, Object> v = byName(list(panache().get("vars")), "NESTED");
        String init = Json.write(v.get("init")).replaceAll("\\s+", "");
        check(init.contains("\"value\":\"\\\"shop.Logs$Part\\\"\""), "NESTED init " + init);
    }

    /**
     * {@code Pair} has three constructors and one field initializer the reader refuses. Java has the
     * construct once; the refusal list should hold it once, as it does for a class with one
     * constructor, so a count of refusals counts constructs.
     */
    private void aFieldInitializerRefusedIsOneRefusal() throws Exception {
        int line = lineOf(testdata.resolve("project-panache/src/main/java/shop/Pair.java"), "public long b = a + 1;");
        int n = 0;
        for (Object r : list(panache().get("refusals"))) {
            if (("Pair.java:" + line).equals(map(r).get("pos"))) {
                n++;
            }
        }
        check(n == 1, n + " refusals at Pair.java:" + line);
    }

    /** {@code <T> Lone(T)} is refused; Java has no {@code new Lone()}, so the module has no {@code NewLone()} either. */
    private void aClassWhoseOneConstructorIsRefusedGetsNoMadeUpConstructor() throws Exception {
        Map<String, Object> doc = panache();
        refusedAt(doc, "project-panache", "Lone.java", "public <T> Lone(T seed)", "");
        Map<String, Object> made = funcByKeyOrNull(doc, "NewLone");
        if (made != null) {
            check(!list(made.get("params")).isEmpty(), "a NewLone() without parameters was made up: " + Json.write(made));
        }
    }

    /**
     * {@code @Entity} on {@code Marks.Oops}, a class read as no struct: the project read neither
     * refuses the annotation nor records it, so it is lost without a trace.
     */
    private void anEntityAnnotationOnAnExceptionClassIsRefusedOrRecorded() throws Exception {
        Map<String, Object> doc = panache();
        int line = lineOf(testdata.resolve("project-panache/src/main/java/shop/Marks.java"), "@Entity");
        boolean refused = false;
        for (Object r : list(doc.get("refusals"))) {
            refused |= ("Marks.java:" + line).equals(map(r).get("pos")) || String.valueOf(map(r).get("pos")).equals("Marks.java:" + (line + 1));
        }
        boolean recorded = false;
        for (Object m : list(doc.get("persistence"))) {
            recorded |= "Oops".equals(map(m).get("at"));
        }
        check(refused || recorded, "@Entity on Oops neither refused nor recorded; persistence "
            + Json.write(list(doc.get("persistence"))).replaceAll("\\s+", " "));
    }

    /**
     * {@code Keeper.helper} injects {@code Svc.Helper}, a scoped class nested in an interface. The
     * reader does not read it (the interface is refused, and only classes nested in a class are
     * read), so the module lists no bean {@code Helper}; an injection point naming it as its {@code
     * bean} names a bean of no read package.
     */
    private void anInjectionPointNamesOnlyABeanTheModuleLists() throws Exception {
        Map<String, Object> doc = panache();
        Set<String> beans = new TreeSet<>();
        for (Object b : list(doc.get("beans"))) {
            beans.add(doc.get("path") + "." + map(b).get("name"));
        }
        List<String> bad = new ArrayList<>();
        for (Object i : list(doc.get("injects"))) {
            Object bean = map(i).get("bean");
            if (bean != null && !((String) bean).isEmpty() && !beans.contains((String) bean)) {
                bad.add(map(i).get("at") + " -> " + bean);
            }
        }
        check(bad.isEmpty(), "injection points naming a bean the module does not list: " + bad + "; beans " + beans);
    }

    /**
     * {@code Use.nested(Outer.Nested n)}: the module {@code app.lib} declares the static nested class
     * as the struct {@code Nested}. A type of {@code app} naming it must name that type.
     */
    private void anExternTypeOfAnotherPackageNamesATypeItDeclares() throws Exception {
        Map<String, Map<String, Object>> mods = packages();
        Set<String> declared = new TreeSet<>();
        for (Object t : list(mods.get("app.lib").get("types"))) {
            declared.add((String) map(t).get("name"));
        }
        List<String> bad = new ArrayList<>();
        for (Map<String, Object> x : externs(mods.get("app"), false)) {
            if ("app.lib".equals(x.get("path")) && !declared.contains((String) x.get("name"))) {
                bad.add((String) x.get("name"));
            }
        }
        check(bad.isEmpty(), "app names types of app.lib it does not declare: " + new TreeSet<>(bad) + "; app.lib declares " + declared);
    }

    /**
     * {@code new Chain()} in {@code app}: {@code app.lib} refuses {@code Chain()} (it calls {@code
     * this(7)}) and makes no {@code NewChain}. The call must be refused, not an extern of a function
     * no module declares.
     */
    private void anExternCallOfAnotherPackageNamesAFunctionItDeclares() throws Exception {
        Map<String, Map<String, Object>> mods = packages();
        Set<String> declared = new TreeSet<>();
        for (Object f : list(mods.get("app.lib").get("funcs"))) {
            if (!map(f).containsKey("recv")) {
                declared.add((String) map(f).get("name"));
            }
        }
        List<String> bad = new ArrayList<>();
        for (Map<String, Object> x : externs(mods.get("app"), true)) {
            if ("app.lib".equals(x.get("path")) && !declared.contains((String) x.get("name"))) {
                bad.add((String) x.get("name"));
            }
        }
        check(bad.isEmpty(), "app calls functions of app.lib it does not declare: " + new TreeSet<>(bad) + "; app.lib declares " + declared);
    }

    /**
     * {@code bump(Thing t) { t.count = t.count + 1; }} with {@code Thing} of {@code app.lib}: Java
     * changes the caller's object. The read gives {@code t} the type {@code extern}, the opaque type
     * whose values the Go reader gives a Go struct passed by value (and whose fields it refuses to
     * read); a write through it changes a copy. The write is refused, or {@code t} is a pointer.
     */
    private void aFieldWrittenThroughAnObjectOfAnotherPackageIsRefusedOrAPointer() throws Exception {
        Map<String, Object> app = packages().get("app");
        int line = lineOf(testdata.resolve("project-packages/src/main/java/app/Use.java"), "t.count = t.count + 1;");
        boolean refused = false;
        for (Object r : list(app.get("refusals"))) {
            refused |= ("Use.java:" + line).equals(map(r).get("pos"));
        }
        Map<String, Object> param = map(list(funcByKey(app, "bump").get("params")).get(0));
        boolean pointer = "ptr".equals(map(param.get("t")).get("k"));
        check(refused || pointer, "the write at Use.java:" + line + " goes through t of type " + Json.write(param.get("t")).replaceAll("\\s+", ""));
    }

    /** {@code new Box(5)} calls the second constructor of {@code app.lib.Box}, {@code NewBox2} there. */
    private void aCallOfAnotherPackagesSecondConstructorNamesItsSecondFunction() throws Exception {
        Map<String, Map<String, Object>> mods = packages();
        check(funcByKeyOrNull(mods.get("app.lib"), "NewBox2") != null, "app.lib has no NewBox2");
        Map<String, Object> body = map(list(funcByKey(mods.get("app"), "Use.boxed").get("body")).get(0));
        String s = Json.write(body).replaceAll("\\s+", "");
        check(s.contains("\"k\":\"extern\",\"path\":\"app.lib\",\"name\":\"NewBox2\""), "boxed " + s);
    }

    // ---- helpers ----

    /** A refusal at the line of {@code file} holding {@code text}, whose reason holds {@code fragment}. */
    private void refusedAt(Map<String, Object> doc, String project, String file, String text, String fragment) throws Exception {
        Path src;
        try (var files = Files.walk(testdata.resolve(project).resolve("src/main/java"))) {
            src = files.filter(f -> f.getFileName().toString().equals(file)).findFirst().orElseThrow();
        }
        String pos = file + ":" + lineOf(src, text);
        for (Object r : list(doc.get("refusals"))) {
            if (pos.equals(map(r).get("pos")) && String.valueOf(map(r).get("reason")).contains(fragment)) {
                return;
            }
        }
        throw new AssertionError("no refusal holding '" + fragment + "' at " + pos + "; refusals "
            + Json.write(doc.get("refusals")).replaceAll("\\s+", " "));
    }

    /** Every {@code extern} object under {@code o}: calls (with {@code args}) or types (without). */
    private static List<Map<String, Object>> externs(Object o, boolean calls) {
        List<Map<String, Object>> out = new ArrayList<>();
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if ("extern".equals(m.get("k")) && m.containsKey("args") == calls) {
                out.add(m);
            }
            for (Object v : m.values()) {
                out.addAll(externs(v, calls));
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                out.addAll(externs(v, calls));
            }
        }
        return out;
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

    private static Map<String, Object> byName(List<Object> xs, String name) {
        for (Object x : xs) {
            if (name.equals(map(x).get("name"))) {
                return map(x);
            }
        }
        throw new AssertionError("no " + name);
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

package codespec.read;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Adversary pass 1 on the Java reader's kit work, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.read.AdversaryTest &lt;repo root&gt; &lt;codespec binary&gt; &lt;work dir&gt; &lt;adapters dir&gt; [case]</pre>
 *
 * The adapters directory holds {@code codespec-read-java}, {@code codespec-run-java}, {@code
 * codespec-write-go} and {@code codespec-run-go}, as {@code task adapters} builds them. Each package
 * under {@code testdata/adversary1} (but {@code project_package_only}) is either refused by the
 * reader, each refusal naming the reason its case expects, or run as Java with {@code
 * codespec-run-java} and read with no refusal, passed, written as Go and run with {@code
 * codespec-run-go} on its {@code calls.json}, the path the reader check takes, printing what the
 * Java program prints. {@code project_package_only} is read as a
 * project and must keep the three refusals the unit's base gives it. It prints one line per case
 * and exits 1 when any case fails.
 */
public final class AdversaryTest {
    private final Path codespec;
    private final Path work;
    private final Path adapters;
    private final Path testdata;

    private AdversaryTest(Path root, Path codespec, Path work, Path adapters) {
        this.codespec = codespec;
        this.work = work;
        this.adapters = adapters;
        this.testdata = root.resolve("adapters/java/testdata/adversary1");
    }

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 4 && args.length != 5) {
            System.err.println("usage: AdversaryTest <repo root> <codespec binary> <work dir> <adapters dir> [case]");
            System.exit(2);
        }
        AdversaryTest t = new AdversaryTest(Path.of(args[0]), Path.of(args[1]), Path.of(args[2]), Path.of(args[3]));
        Files.createDirectories(t.work);
        Map<String, Case> cases = new LinkedHashMap<>();
        cases.put("an_int_literal_shifted_by_a_variable_count_is_refused",
            () -> t.readRefuses("shift_untyped", 3, "a shift of a literal by a count that is not constant"));
        cases.put("arithmetic_on_float_literals_is_refused",
            () -> t.readRefuses("float_constants", 3, "of two floating-point constants"));
        cases.put("a_negated_float_literal_is_refused",
            () -> t.readRefuses("negative_zero", 2, "- of a floating-point constant"));
        cases.put("a_compound_assignment_to_an_element_a_field_or_a_global_with_a_call_is_refused",
            () -> t.readRefuses("compound_order", 3, "Java reads the target before the call"));
        cases.put("a_static_field_named_through_its_class_beside_a_local_of_its_name_is_refused",
            () -> t.readRefuses("global_shadow", 3, "a module-level variable and a local of one name"));
        cases.put("a_wrapping_int_constant_prints_as_java_folds_it", () -> t.readPrintsAsJava("println_constant"));
        cases.put("a_second_value_assigned_to_a_local_of_an_interface_is_refused",
            () -> t.readRefuses("iface_reassign", 1, "a local of interface"));
        cases.put("a_project_read_refuses_the_shift_the_labeled_break_and_the_floats_as_before",
            t::projectReadIsAsBefore);
        String only = args.length == 5 ? args[4] : null;
        int passed = 0;
        int failed = 0;
        for (Map.Entry<String, Case> c : cases.entrySet()) {
            if (only != null && !c.getKey().equals(only)) {
                continue;
            }
            try {
                c.getValue().run();
                System.out.println("ok   " + c.getKey());
                passed++;
            } catch (Exception | AssertionError e) {
                System.out.println("FAIL " + c.getKey() + ": " + e.getMessage());
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed");
        if (failed > 0 || passed == 0) {
            System.exit(1);
        }
    }

    /**
     * The package {@code name} read with exactly {@code count} refusals, each naming {@code reason}:
     * the reader refuses what it would otherwise read to a program printing other than Java.
     */
    private void readRefuses(String name, int count, String reason) throws Exception {
        Path dir = testdata.resolve(name);
        Path w = work.resolve(name);
        Files.createDirectories(w);
        Path read = w.resolve("read.cir.json");
        run("codespec read", w.resolve("stdout.txt"), w.resolve("stderr.txt"), codespec.toString(), "read", "--lang",
            "java", dir.toString(), "--out", read.toString(), "--adapters", adapters.toString());
        Map<String, Object> doc = map(Json.parse(Files.readString(read)));
        List<Object> rs = doc.containsKey("refusals") ? list(doc.get("refusals")) : new ArrayList<>();
        boolean all = rs.size() == count;
        for (Object r : rs) {
            all &= String.valueOf(map(r).get("reason")).contains(reason);
        }
        if (!all) {
            throw new AssertionError("want " + count + " refusals naming '" + reason + "': "
                + Json.write(rs).replaceAll("\\s+", " "));
        }
    }

    /**
     * The package {@code name} run as Java, and read with no refusal, passed, written as Go and run:
     * it prints, call for call, what the Java program prints.
     */
    private void readPrintsAsJava(String name) throws Exception {
        Path dir = testdata.resolve(name);
        Path calls = dir.resolve("calls.json");
        Path w = work.resolve(name);
        Files.createDirectories(w);
        Path err = w.resolve("stderr.txt");
        Path log = w.resolve("stdout.txt");
        Path javaOut = w.resolve("java.txt");
        run("codespec-run-java", javaOut, err, adapters.resolve("codespec-run-java").toString(), "--build",
            w.resolve("java-build").toString(), dir.toString(), calls.toString());
        Path read = w.resolve("read.cir.json");
        run("codespec read", log, err, codespec.toString(), "read", "--lang", "java", dir.toString(), "--out",
            read.toString(), "--adapters", adapters.toString());
        Map<String, Object> doc = map(Json.parse(Files.readString(read)));
        if (doc.containsKey("refusals")) {
            throw new AssertionError("refusals: " + Json.write(doc.get("refusals")).replaceAll("\\s+", " "));
        }
        Path passedDoc = w.resolve("passed.cir.json");
        run("codespec pass", log, err, codespec.toString(), "pass", read.toString(), "--out", passedDoc.toString());
        run("codespec write --lang go", log, err, codespec.toString(), "write", "--lang", "go", passedDoc.toString(),
            "--out", w.resolve("go").toString(), "--adapters", adapters.toString());
        Path goOut = w.resolve("go.txt");
        run("codespec-run-go", goOut, err, adapters.resolve("codespec-run-go").toString(), "-build",
            w.resolve("go-build").toString(), w.resolve("go").toString(), calls.toString());
        List<String> want = Files.readAllLines(javaOut, StandardCharsets.UTF_8);
        List<String> have = Files.readAllLines(goOut, StandardCharsets.UTF_8);
        for (int i = 0; i < Math.max(want.size(), have.size()); i++) {
            String j = i < want.size() ? want.get(i) : "<none>";
            String g = i < have.size() ? have.get(i) : "<none>";
            if (!j.equals(g)) {
                throw new AssertionError("read without refusal, line " + (i + 1) + ": Java printed `" + j
                    + "`, the Go written from the read printed `" + g + "`");
            }
        }
    }

    /**
     * {@code project_package_only} read as a project: its shift, labeled break and float literal
     * compare are each refused, as at the unit's base, since the kit mappings are a package read's
     * only (README.md, "each only in a package read (a project's modules are read as before)").
     */
    private void projectReadIsAsBefore() throws Exception {
        Path dir = testdata.resolve("project_package_only");
        Path w = work.resolve("project_package_only");
        Files.createDirectories(w);
        Path out = w.resolve("read.cir.json");
        run("codespec-read-java", out, w.resolve("stderr.txt"), adapters.resolve("codespec-read-java").toString(), dir.toString());
        Map<String, Object> doc = map(Json.parse(Files.readString(out)));
        List<Object> rs = doc.containsKey("refusals") ? list(doc.get("refusals")) : new ArrayList<>();
        if (rs.size() != 3) {
            throw new AssertionError("a project read gives " + rs.size() + " refusals, the unit's base 3 (shift, labeled"
                + " statement, floating-point values): " + Json.write(rs).replaceAll("\\s+", " "));
        }
    }

    private static void run(String step, Path out, Path err, String... cmd) throws Exception {
        Process p = new ProcessBuilder(cmd).redirectOutput(out.toFile()).redirectError(err.toFile()).start();
        int code = p.waitFor();
        if (code != 0) {
            throw new AssertionError(step + " exited " + code + ": " + Files.readString(err).strip());
        }
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> map(Object o) {
        return (Map<String, Object>) o;
    }

    @SuppressWarnings("unchecked")
    private static List<Object> list(Object o) {
        return (List<Object>) o;
    }
}

package codespec.read;

import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

/**
 * {@code codespec-read-java <package dir>}: reads every {@code .java} file of one package with
 * javac's attributed trees and prints it as one cir/1 module on stdout. Constructs cir/1 cannot
 * express are kept as {@code foreign} statements and listed as refusals.
 *
 * <p>{@code codespec-read-java <project dir>}: a directory with {@code pom.xml} or {@code
 * build.gradle(.kts)} is a project. Every package under {@code src/main/java} is read in one javac
 * analysis against the jars {@code CODESPEC_JAVA_CLASSPATH} names (separated by {@code :}), one
 * module per package. A project of one package prints its module; of several, {@code
 * CODESPEC_JAVA_PACKAGE} names the one to print. The reader never runs Maven or Gradle.
 *
 * <p>{@code codespec-read-java --packages <project dir>} lists the packages, one per line.
 * {@code codespec-read-java --all <project dir> <out dir>} writes every package's module to {@code
 * <out dir>/<package>.cir.json}. {@code codespec-read-java --effects <project dir>} prints the
 * codespec-effects/1 summary of the project's REST clients ({@code net}) and of the Panache, entity
 * manager and JDBC calls ({@code db}).
 *
 * <p>{@code codespec-read-java --describe} prints the reader's codespec-adapter/1 manifest.
 */
public final class Main {
    /** The workspace version; written into every document as {@code reader.version}. */
    public static final String VERSION = "0.9.0";

    /**
     * The reader's claim: the features its output for {@code examples/ledger} uses and the subject
     * of every conformance kit case with a {@code java/Case.java}, which the reader reads, the passes
     * and the Go writer write and {@code codespec-run-go} runs to the case's {@code expected.txt}, or,
     * for a module the Go writer refuses ({@code decl.union}), {@code codespec-write-cir} writes and
     * {@code codespec-run-cir} runs ({@code ReaderTest}; {@code codespec-conformance manifest
     * --read} over the ledger and the kit modules checks every claimed feature is used).
     */
    static final String[] CLAIM = {
        "assign.add", "assign.and", "assign.div", "assign.mul", "assign.or", "assign.rem",
        "assign.shl", "assign.shr", "assign.sub", "assign.xor", "decl.enum", "decl.interface",
        "decl.struct", "decl.type_params", "decl.union", "decl.var", "decl.variadic", "expr.binop", "expr.call",
        "expr.call_value", "expr.const", "expr.conv", "expr.errvar", "expr.field", "expr.float",
        "expr.func_ref", "expr.global", "expr.iface_call", "expr.index", "expr.instantiate",
        "expr.intrinsic", "expr.lambda", "expr.lit", "expr.make", "expr.method", "expr.nil",
        "expr.spread", "expr.struct", "expr.to_error", "expr.to_iface", "expr.unop", "expr.var", "expr.zero",
        "intrinsic.append", "intrinsic.errorf", "intrinsic.len", "intrinsic.println",
        "intrinsic.sorted_keys", "intrinsic.sprintf", "op.add", "op.and", "op.div", "op.eq", "op.ge",
        "op.gt", "op.land", "op.le", "op.lor", "op.lt", "op.mul", "op.ne", "op.or", "op.rem",
        "op.shl", "op.shr", "op.sub", "op.xor", "sem.continue.post", "sem.error.propagate",
        "sem.int.wrap", "sem.map.order", "sem.rem.sign", "sem.switch.break", "stmt.assign",
        "stmt.break", "stmt.continue", "stmt.decr", "stmt.expr", "stmt.for", "stmt.for_range",
        "stmt.if", "stmt.incr", "stmt.let", "stmt.lookup_else", "stmt.return", "stmt.switch", "type.any", "type.bool",
        "type.error", "type.float", "type.func", "type.int", "type.map", "type.named", "type.param",
        "type.ptr", "type.slice", "type.string", "type.tuple", "type.unit", "unop.complement",
        "unop.neg", "unop.not", "unop.plus", "unop.ref",
    };

    private Main() {}

    /**
     * The codespec-adapter/1 manifest, written by hand in the canonical encoding of crates/model:
     * two-space indentation, a final newline.
     */
    static String describe() {
        StringBuilder b = new StringBuilder("{\n");
        b.append("  \"format\": \"codespec-adapter/1\",\n");
        b.append("  \"role\": \"reader\",\n");
        b.append("  \"lang\": \"java\",\n");
        b.append("  \"version\": \"").append(VERSION).append("\",\n");
        b.append("  \"cir\": \"cir/1\",\n");
        b.append("  \"features\": [\n");
        for (int i = 0; i < CLAIM.length; i++) {
            b.append("    \"").append(CLAIM[i]).append('"');
            b.append(i < CLAIM.length - 1 ? ",\n" : "\n");
        }
        b.append("  ]\n}\n");
        return b.toString();
    }

    private static final String USAGE =
        "usage: codespec-read-java <package or project dir> | --packages <project dir> | --all <project dir> <out dir>"
            + " | --effects <project dir> | --describe";

    /** Runs the reader on the stack {@link codespec.jvm.Bounded} states; deeper is a refusal. */
    public static void main(String[] args) {
        codespec.jvm.Bounded.run("codespec-read-java", () -> run(args));
    }

    private static void run(String[] args) {
        if (args.length == 1 && args[0].equals("--describe")) {
            print(describe());
            return;
        }
        try {
            if (args.length == 2 && args[0].equals("--packages")) {
                StringBuilder b = new StringBuilder();
                for (String p : Reader.packages(Path.of(args[1]))) {
                    b.append(p).append('\n');
                }
                print(b.toString());
            } else if (args.length == 2 && args[0].equals("--effects")) {
                print(Reader.effectsSummary(Path.of(args[1]), classpath()));
            } else if (args.length == 3 && args[0].equals("--all")) {
                Path outDir = Path.of(args[2]);
                Files.createDirectories(outDir);
                for (Map.Entry<String, Map<String, Object>> e : Reader.readProject(Path.of(args[1]), classpath()).entrySet()) {
                    Files.writeString(outDir.resolve(e.getKey() + ".cir.json"), Json.write(e.getValue()), StandardCharsets.UTF_8);
                }
            } else if (args.length == 1 && !args[0].startsWith("--")) {
                print(Json.write(read(Path.of(args[0]))));
            } else {
                System.err.println(USAGE);
                System.exit(2);
            }
        } catch (Reader.Failure | IOException e) {
            System.err.println("codespec-read-java: " + e.getMessage());
            System.exit(1);
        }
    }

    /** One module: a package directory's, or the one package of a project that the environment names. */
    private static Map<String, Object> read(Path dir) throws Reader.Failure {
        if (!Reader.isProject(dir)) {
            return Reader.read(dir);
        }
        Map<String, Map<String, Object>> modules = Reader.readProject(dir, classpath());
        String want = System.getenv("CODESPEC_JAVA_PACKAGE");
        if (want == null || want.isEmpty()) {
            if (modules.size() == 1) {
                return modules.values().iterator().next();
            }
            throw new Reader.Failure(dir + " has " + modules.size() + " packages (" + String.join(", ", modules.keySet())
                + "): CODESPEC_JAVA_PACKAGE names the one to read");
        }
        Map<String, Object> m = modules.get(want);
        if (m == null) {
            throw new Reader.Failure(dir + " has no package " + want + ", only " + String.join(", ", modules.keySet()));
        }
        return m;
    }

    private static List<Path> classpath() {
        String cp = System.getenv(Reader.CLASSPATH_VARIABLE);
        return cp == null ? List.of() : Reader.classpath(cp);
    }

    private static void print(String s) {
        PrintStream stdout = new PrintStream(System.out, false, StandardCharsets.UTF_8);
        stdout.print(s);
        stdout.flush();
    }
}

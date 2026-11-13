package codespec.read.groovy;

import java.io.File;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * {@code codespec-read-groovy <dir>}: reads every {@code .groovy} file of one directory with the
 * Groovy 2.4 compiler and its static type checker and prints it as one cir/1 module on stdout. A
 * directory holding {@code application.properties} is a Grails 2.5.6 application, whose domain
 * classes and services are read ({@link Grails}); {@code CODESPEC_GROOVY_CLASSPATH} names jars,
 * path-separated, added to the compile classpath.
 * Constructs cir/1 cannot express, and dynamic code the type checker cannot type, are kept as {@code
 * foreign} statements and listed as refusals, each reason beginning with its category.
 *
 * <p>{@code codespec-read-groovy --describe} prints the reader's codespec-adapter/1 manifest.
 */
public final class Main {
    /** The workspace version; written into every document as {@code reader.version}. */
    public static final String VERSION = "0.10.0";

    /**
     * The reader's claim: every feature whose kit cases in the Groovy corpus {@code
     * conformance/reader/groovy} all pass {@code codespec conform}, each used by the reader's output
     * for {@code adapters/groovy/testdata/typed} or for that corpus ({@code codespec-conformance
     * manifest --read} checks it). An integer {@code /} is a BigDecimal division in Groovy, which
     * the reader refuses; {@code op.div}, {@code assign.div} and {@code sem.int.wrap} are claimed
     * through {@code intdiv}, which their cases' {@code groovy/Case.groovy} sources divide with.
     */
    static final String[] CLAIM = {
        "assign.add", "assign.and", "assign.div", "assign.mul", "assign.or", "assign.rem", "assign.sub",
        "assign.xor", "decl.enum", "decl.struct", "expr.binop", "expr.call", "expr.const",
        "expr.conv", "expr.field", "expr.index", "expr.intrinsic", "expr.lit", "expr.make",
        "expr.method", "expr.struct", "expr.unop", "expr.var", "intrinsic.append", "intrinsic.len",
        "intrinsic.sorted_keys", "intrinsic.sprintf", "op.add", "op.and", "op.div", "op.eq", "op.ge", "op.gt",
        "op.land", "op.le", "op.lor", "op.lt", "op.mul", "op.ne", "op.or", "op.rem", "op.sub",
        "op.xor", "sem.continue.post", "sem.int.wrap", "sem.map.order", "sem.rem.sign", "stmt.assign", "stmt.break",
        "stmt.continue", "stmt.decr", "stmt.expr", "stmt.for", "stmt.for_range", "stmt.if",
        "stmt.incr", "stmt.let", "stmt.return", "stmt.switch", "type.bool", "type.int", "type.map",
        "type.named", "type.ptr", "type.slice", "type.string", "type.unit", "unop.complement",
        "unop.neg", "unop.not", "unop.plus", "unop.ref",
    };

    /** The variable naming the jars, path-separated, added to the Groovy jar on the compile classpath. */
    static final String CLASSPATH_ENV = "CODESPEC_GROOVY_CLASSPATH";

    private Main() {}

    /** The jars a {@code CODESPEC_GROOVY_CLASSPATH} value names, none for {@code null} or empty. */
    static List<Path> classpath(String value) {
        List<Path> out = new ArrayList<>();
        if (value == null) {
            return out;
        }
        for (String part : value.split(File.pathSeparator)) {
            if (!part.isEmpty()) {
                out.add(Path.of(part));
            }
        }
        return out;
    }

    /**
     * Read {@code dir}: a Grails 2.5.6 application when it holds {@code application.properties},
     * else the {@code .groovy} files directly in it, all of one package.
     */
    static Map<String, Object> read(Path dir, List<Path> classpath) throws Reader.Failure {
        if (Grails.isApplication(dir)) {
            return Reader.read(Grails.of(dir), classpath);
        }
        return Reader.read(dir);
    }

    /**
     * The codespec-adapter/1 manifest, written by hand in the canonical encoding of crates/model:
     * two-space indentation, a final newline.
     */
    static String describe() {
        StringBuilder b = new StringBuilder("{\n");
        b.append("  \"format\": \"codespec-adapter/1\",\n");
        b.append("  \"role\": \"reader\",\n");
        b.append("  \"lang\": \"groovy\",\n");
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

    /** Runs the reader on the stack {@link codespec.jvm.Bounded} states; deeper is a refusal. */
    public static void main(String[] args) {
        codespec.jvm.Bounded.run("codespec-read-groovy", () -> run(args));
    }

    private static void run(String[] args) {
        PrintStream stdout = new PrintStream(System.out, false, StandardCharsets.UTF_8);
        if (args.length == 1 && args[0].equals("--describe")) {
            stdout.print(describe());
            stdout.flush();
            return;
        }
        if (args.length != 1) {
            System.err.println("usage: codespec-read-groovy <dir> | --describe");
            System.exit(2);
        }
        String out;
        try {
            out = Json.write(read(Path.of(args[0]), classpath(System.getenv(CLASSPATH_ENV))));
        } catch (Reader.Failure e) {
            System.err.println("codespec-read-groovy: " + e.getMessage());
            System.exit(1);
            return;
        }
        stdout.print(out);
        stdout.flush();
    }
}

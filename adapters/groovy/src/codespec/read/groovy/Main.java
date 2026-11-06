package codespec.read.groovy;

import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;

/**
 * {@code codespec-read-groovy <dir>}: reads every {@code .groovy} file of one directory with the
 * Groovy 2.4 compiler and its static type checker and prints it as one cir/1 module on stdout.
 * Constructs cir/1 cannot express, and dynamic code the type checker cannot type, are kept as {@code
 * foreign} statements and listed as refusals, each reason beginning with its category.
 *
 * <p>{@code codespec-read-groovy --describe} prints the reader's codespec-adapter/1 manifest.
 */
public final class Main {
    /** The workspace version; written into every document as {@code reader.version}. */
    public static final String VERSION = "0.2.0";

    /**
     * The reader's claim: the features its output for {@code adapters/groovy/testdata/typed} uses
     * ({@code codespec-conformance manifest --read} checks it).
     */
    static final String[] CLAIM = {
        "decl.enum", "decl.struct", "expr.binop", "expr.const", "expr.conv", "expr.field",
        "expr.index", "expr.intrinsic", "expr.lit", "expr.make", "expr.struct", "expr.unop",
        "expr.var", "intrinsic.append", "intrinsic.len", "intrinsic.sorted_keys", "intrinsic.sprintf",
        "op.add", "op.eq", "op.gt", "op.land", "op.lt", "op.mul", "sem.int.wrap", "sem.map.order",
        "stmt.assign", "stmt.break", "stmt.for", "stmt.for_range", "stmt.if", "stmt.incr",
        "stmt.let", "stmt.return", "stmt.switch", "type.bool", "type.int", "type.map", "type.named",
        "type.ptr", "type.slice", "type.string", "unop.ref",
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

    public static void main(String[] args) {
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
            out = Json.write(Reader.read(Path.of(args[0])));
        } catch (Reader.Failure e) {
            System.err.println("codespec-read-groovy: " + e.getMessage());
            System.exit(1);
            return;
        }
        stdout.print(out);
        stdout.flush();
    }
}

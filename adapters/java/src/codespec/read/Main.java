package codespec.read;

import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;

/**
 * {@code codespec-read-java <package dir>}: reads every {@code .java} file of one package with
 * javac's attributed trees and prints it as one cir/1 module on stdout. Constructs cir/1 cannot
 * express are kept as {@code foreign} statements and listed as refusals.
 *
 * <p>{@code codespec-read-java --describe} prints the reader's codespec-adapter/1 manifest.
 */
public final class Main {
    /** The workspace version; written into every document as {@code reader.version}. */
    public static final String VERSION = "0.2.0";

    /**
     * The reader's claim: the features its output for {@code examples/ledger} uses, the only Java
     * the repository reads end to end ({@code codespec-conformance manifest --read} checks it).
     */
    static final String[] CLAIM = {
        "assign.add", "assign.sub", "decl.enum", "decl.struct", "expr.binop", "expr.call",
        "expr.const", "expr.conv", "expr.errvar", "expr.field", "expr.index", "expr.intrinsic",
        "expr.lit", "expr.make", "expr.method", "expr.nil", "expr.struct", "expr.unop", "expr.var",
        "expr.zero", "intrinsic.append", "intrinsic.len", "intrinsic.sorted_keys",
        "intrinsic.sprintf", "op.add", "op.eq", "op.ge", "op.gt", "op.land", "op.le", "op.lt",
        "op.ne", "op.sub", "sem.int.wrap", "sem.map.order", "stmt.assign", "stmt.break",
        "stmt.expr", "stmt.for", "stmt.for_range", "stmt.if", "stmt.incr", "stmt.let",
        "stmt.return", "stmt.switch", "type.bool", "type.error", "type.int", "type.map",
        "type.named", "type.ptr", "type.slice", "type.string", "type.unit", "unop.ref",
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

    public static void main(String[] args) {
        if (args.length == 1 && args[0].equals("--describe")) {
            PrintStream stdout = new PrintStream(System.out, false, StandardCharsets.UTF_8);
            stdout.print(describe());
            stdout.flush();
            return;
        }
        if (args.length != 1) {
            System.err.println("usage: codespec-read-java <package dir> | --describe");
            System.exit(2);
        }
        String out;
        try {
            out = Json.write(Reader.read(Path.of(args[0])));
        } catch (Reader.Failure e) {
            System.err.println("codespec-read-java: " + e.getMessage());
            System.exit(1);
            return;
        }
        PrintStream stdout = new PrintStream(System.out, false, StandardCharsets.UTF_8);
        stdout.print(out);
        stdout.flush();
    }
}

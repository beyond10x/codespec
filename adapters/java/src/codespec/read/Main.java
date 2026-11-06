package codespec.read;

import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;

/**
 * {@code codespec-read-java <package dir>}: reads every {@code .java} file of one package with
 * javac's attributed trees and prints it as one cir/1 module on stdout. Constructs cir/1 cannot
 * express are kept as {@code foreign} statements and listed as refusals.
 */
public final class Main {
    /** The workspace version; written into every document as {@code reader.version}. */
    public static final String VERSION = "0.2.0";

    private Main() {}

    public static void main(String[] args) {
        if (args.length != 1) {
            System.err.println("usage: codespec-read-java <package dir>");
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

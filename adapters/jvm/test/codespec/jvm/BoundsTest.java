package codespec.jvm;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;

/**
 * What a deeply nested source costs the JVM readers, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.jvm.BoundsTest &lt;adapters dir&gt; &lt;work dir&gt;</pre>
 *
 * <p>Neither javac nor Groovy's parser bounds nesting depth: each recurses a level per
 * parenthesis until the stack runs out. Each reader is run on a generated source nested a million
 * parentheses deep and must end in exit 0 or a refusal, exit 1 with one line naming the reader,
 * never a crash (a JVM error and its stack trace). It prints one line per case and a summary
 * {@code N passed, M failed}, and exits 1 when any case fails.
 */
public final class BoundsTest {
    private BoundsTest() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 2) {
            System.err.println("usage: BoundsTest <adapters dir> <work dir>");
            System.exit(2);
        }
        Path adapters = Path.of(args[0]);
        Path work = Path.of(args[1]);
        int n = 1_000_000;
        String body = "(".repeat(n) + "a" + ")".repeat(n);
        String java = "package p;\n\npublic class P {\n    public static int f(int a) {\n        return " + body
            + ";\n    }\n}\n";
        String groovy = "package p\n\nclass P {\n    static int f(int a) {\n        return " + body + "\n    }\n}\n";
        int passed = 0;
        int failed = 0;
        String[][] cases = {
            {"codespec-read-java", "P.java", java},
            {"codespec-read-groovy", "P.groovy", groovy},
        };
        for (String[] c : cases) {
            String name = c[0] + "_ends_a_million_parentheses_in_exit_0_or_a_refusal";
            try {
                check(adapters.resolve(c[0]), work.resolve(c[0]), c[1], c[2]);
                System.out.println("ok   " + name);
                passed++;
            } catch (Throwable e) {
                System.out.println("FAIL " + name + ": " + e.getMessage());
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed");
        System.exit(failed == 0 ? 0 : 1);
    }

    private static void check(Path reader, Path work, String file, String text) throws Exception {
        Path dir = work.resolve("p");
        Files.createDirectories(dir);
        Files.writeString(dir.resolve(file), text, StandardCharsets.UTF_8);
        Path out = work.resolve("stdout");
        Path err = work.resolve("stderr");
        Process p = new ProcessBuilder(reader.toString(), dir.toString())
            .redirectOutput(out.toFile())
            .redirectError(err.toFile())
            .start();
        if (!p.waitFor(10, TimeUnit.MINUTES)) {
            p.destroyForcibly();
            throw new AssertionError("still running after 10 minutes");
        }
        String stderr = Files.readString(err, StandardCharsets.UTF_8);
        String prefix = reader.getFileName() + ": ";
        boolean refusal = p.exitValue() == 1
            && stderr.startsWith(prefix)
            && stderr.indexOf('\n') == stderr.length() - 1
            && Files.size(out) == 0;
        if (p.exitValue() != 0 && !refusal) {
            String head = stderr.length() > 600 ? stderr.substring(0, 600) + " …" : stderr;
            throw new AssertionError("exit " + p.exitValue() + ", not 0 or a one-line refusal:\n" + head);
        }
    }
}

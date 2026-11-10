package codespec.read;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

/**
 * Adversary pass 1 on story:cir1-raise-out-of-entry. {@code Res.find} is an http entry that throws
 * the project's checked {@code Missing}, which the reader reads as an error result, not a raise:
 * {@code find} lists no {@code raises}. Before the unit, {@code Res.twice}'s {@code long a =
 * find(k);} read with no refusal; the unit's {@code mayRaise} counts any {@code throw} in an entry
 * and refuses the call as one "which raises".
 *
 * <p>Usage: {@code java -cp <classes> codespec.read.AdversaryRaiseTest <work dir>} with {@code
 * CODESPEC_JAVA_CLASSPATH} set as for ReaderTest. Exit 1 when red.
 */
public final class AdversaryRaiseTest {
    private AdversaryRaiseTest() {}

    public static void main(String[] args) throws Exception {
        Path dir = Path.of(args[0]).resolve("project-checked");
        Path pkg = dir.resolve("src/main/java/adv2");
        Files.createDirectories(pkg);
        Files.writeString(dir.resolve("pom.xml"), "<project/>\n", StandardCharsets.UTF_8);
        Files.writeString(pkg.resolve("AdvError.java"), String.join("\n",
            "package adv2;",
            "",
            "public class AdvError extends Exception {",
            "    public AdvError(String message) {",
            "        super(message);",
            "    }",
            "}",
            ""), StandardCharsets.UTF_8);
        Files.writeString(pkg.resolve("Missing.java"), String.join("\n",
            "package adv2;",
            "",
            "public class Missing extends AdvError {",
            "    public Missing() {",
            "        super(\"missing\");",
            "    }",
            "}",
            ""), StandardCharsets.UTF_8);
        Files.writeString(pkg.resolve("Res.java"), String.join("\n",
            "package adv2;",
            "",
            "import jakarta.ws.rs.POST;",
            "import jakarta.ws.rs.Path;",
            "",
            "@Path(\"res\")",
            "public class Res {",
            "    @POST",
            "    public long find(long k) throws Missing {",
            "        if (k < 0) {",
            "            throw new Missing();",
            "        }",
            "        return k;",
            "    }",
            "",
            "    long twice(long k) throws Missing {",
            "        long a = find(k);",
            "        return a + 1;",
            "    }",
            "}",
            ""), StandardCharsets.UTF_8);
        Map<String, Object> doc = Reader.readProject(dir, Reader.classpath(System.getenv("CODESPEC_JAVA_CLASSPATH")))
            .get("adv2");
        Object refusals = doc.get("refusals");
        boolean red = false;
        for (Object f : (List<?>) doc.get("funcs")) {
            Map<?, ?> fn = (Map<?, ?>) f;
            if ("find".equals(fn.get("name")) && fn.containsKey("raises")) {
                System.out.println("find lists raises: " + Json.write(fn.get("raises")));
            }
        }
        if (refusals != null && !((List<?>) refusals).isEmpty()) {
            System.out.println("FAIL adversary_a_call_of_an_entry_that_throws_a_checked_exception_reads_as_before: "
                + Json.write(refusals).replaceAll("\\s+", " "));
            red = true;
        }
        if (red) {
            System.exit(1);
        }
        System.out.println("ok adversary_a_call_of_an_entry_that_throws_a_checked_exception_reads_as_before");
    }
}

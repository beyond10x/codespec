package codespec.read.groovy;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The shared JSON writer's tests, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.read.groovy.JsonTest</pre>
 *
 * It prints one line per case and a summary {@code N passed, M failed}, and exits 1 when any case
 * fails.
 */
public final class JsonTest {
    private JsonTest() {}

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) {
        Map<String, Case> cases = new LinkedHashMap<>();
        cases.put("a_document_is_written_in_the_layout_serde_json_pretty_prints", JsonTest::layout);
        cases.put("control_characters_quotes_and_backslashes_are_escaped_as_serde_json_escapes_them", JsonTest::escapes);
        cases.put("a_written_document_parses_back_to_the_same_bytes", JsonTest::roundTrip);
        int passed = 0;
        int failed = 0;
        for (Map.Entry<String, Case> c : cases.entrySet()) {
            try {
                c.getValue().run();
                System.out.println("ok   " + c.getKey());
                passed++;
            } catch (Throwable e) {
                System.out.println("FAIL " + c.getKey() + ": " + e);
                failed++;
            }
        }
        System.out.println(passed + " passed, " + failed + " failed");
        System.exit(failed == 0 ? 0 : 1);
    }

    private static void layout() {
        Map<String, Object> doc = Json.obj();
        doc.put("format", "cir/1");
        doc.put("empty", new ArrayList<Object>());
        doc.put("none", Json.obj());
        List<Object> xs = new ArrayList<>();
        xs.add(Boolean.TRUE);
        xs.add("b");
        doc.put("xs", xs);
        String want = "{\n  \"format\": \"cir/1\",\n  \"empty\": [],\n  \"none\": {},\n  \"xs\": [\n    true,\n    \"b\"\n  ]\n}\n";
        check(Json.write(doc).equals(want), "got " + Json.write(doc));
    }

    private static void escapes() {
        String got = Json.write("a\"b\\c\nd\u0001");
        check(got.equals("\"a\\\"b\\\\c\\nd\\u0001\"\n"), "got " + got);
    }

    private static void roundTrip() {
        String src = "{\"k\":\"lit\",\"value\":\"\\\"x\\\"\",\"t\":{\"k\":\"string\"},\"list\":[false,{}]}";
        String once = Json.write(Json.parse(src));
        check(Json.write(Json.parse(once)).equals(once), "not a fixed point: " + once);
    }

    private static void check(boolean ok, String what) {
        if (!ok) {
            throw new AssertionError(what);
        }
    }
}

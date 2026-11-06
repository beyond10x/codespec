import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Runs the original Java of the nullable fixture on a codespec-calls/1 document and prints one line
 * per call in the harness encoding, as codespec-run-go and codespec-run-rust print the written Go
 * and Rust:
 *
 * <pre>java -cp &lt;classes&gt; Main &lt;calls.json&gt;</pre>
 *
 * A call's {@code func} is a public static method of {@code nullable.Note} with its first letter
 * upper-cased, as the Go and Rust writers export it. A String argument is a double-quoted string of
 * printable ASCII other than {@code "} and {@code \}; an integer a decimal; a boolean {@code true}
 * or {@code false}. The line is {@code ok} for a method without a result and {@code ok <v>}
 * otherwise, a String as Go's {@code strconv.Quote} writes printable ASCII.
 */
public final class Main {
    private Main() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            System.err.println("usage: Main <calls.json>");
            System.exit(2);
        }
        Map<String, Object> doc = obj(new Json(Files.readString(Path.of(args[0]), StandardCharsets.UTF_8)).document());
        if (!"codespec-calls/1".equals(doc.get("format"))) {
            throw new IllegalArgumentException("not codespec-calls/1: " + doc.get("format"));
        }
        Class<?> subject = Class.forName("nullable.Note");
        StringBuilder out = new StringBuilder();
        for (Object c : list(doc.get("calls"))) {
            Map<String, Object> call = obj(c);
            String func = (String) call.get("func");
            List<Object> as = list(call.get("args"));
            Method m = method(subject, func, as.size());
            Object[] values = new Object[as.size()];
            for (int i = 0; i < values.length; i++) {
                values[i] = argument(m.getParameterTypes()[i], (String) as.get(i));
            }
            Object r;
            try {
                r = m.invoke(null, values);
            } catch (InvocationTargetException e) {
                throw new IllegalStateException(func + " threw " + e.getCause(), e.getCause());
            }
            out.append(m.getReturnType() == void.class ? "ok" : "ok " + value(r)).append('\n');
        }
        System.out.print(out);
    }

    private static Method method(Class<?> subject, String func, int arity) {
        for (Method m : subject.getDeclaredMethods()) {
            String n = m.getName();
            String exported = Character.toUpperCase(n.charAt(0)) + n.substring(1);
            if (Modifier.isStatic(m.getModifiers()) && Modifier.isPublic(m.getModifiers()) && exported.equals(func)
                && m.getParameterCount() == arity) {
                return m;
            }
        }
        throw new IllegalArgumentException("no public static method " + func + " of " + arity + " parameters");
    }

    private static Object argument(Class<?> t, String a) {
        if (t == String.class) {
            if (a.length() < 2 || a.charAt(0) != '"' || a.charAt(a.length() - 1) != '"') {
                throw new IllegalArgumentException("not a string argument: " + a);
            }
            return a.substring(1, a.length() - 1);
        }
        if (t == int.class) {
            return Integer.parseInt(a);
        }
        if (t == long.class) {
            return Long.parseLong(a);
        }
        if (t == boolean.class && (a.equals("true") || a.equals("false"))) {
            return Boolean.parseBoolean(a);
        }
        throw new IllegalArgumentException("argument " + a + " for a parameter of " + t);
    }

    private static String value(Object v) {
        if (v instanceof String) {
            StringBuilder b = new StringBuilder("\"");
            for (char c : ((String) v).toCharArray()) {
                if (c < 0x20 || c > 0x7e) {
                    throw new IllegalArgumentException("a result outside printable ASCII");
                }
                if (c == '"' || c == '\\') {
                    b.append('\\');
                }
                b.append(c);
            }
            return b.append('"').toString();
        }
        return String.valueOf(v);
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> obj(Object o) {
        return (Map<String, Object>) o;
    }

    @SuppressWarnings("unchecked")
    private static List<Object> list(Object o) {
        return (List<Object>) o;
    }

    /** JSON objects, arrays and strings with the escapes a calls document uses. */
    private static final class Json {
        private final String s;
        private int i;

        Json(String s) {
            this.s = s;
        }

        Object document() {
            Object v = value();
            ws();
            if (i != s.length()) {
                throw fail("trailing input");
            }
            return v;
        }

        private IllegalArgumentException fail(String what) {
            return new IllegalArgumentException("calls JSON at " + i + ": " + what);
        }

        private void ws() {
            while (i < s.length() && Character.isWhitespace(s.charAt(i))) {
                i++;
            }
        }

        private boolean take(char c) {
            ws();
            if (i < s.length() && s.charAt(i) == c) {
                i++;
                return true;
            }
            return false;
        }

        private void expect(char c) {
            if (!take(c)) {
                throw fail("want " + c);
            }
        }

        private Object value() {
            if (take('{')) {
                Map<String, Object> m = new LinkedHashMap<>();
                if (take('}')) {
                    return m;
                }
                do {
                    ws();
                    String k = string();
                    expect(':');
                    m.put(k, value());
                } while (take(','));
                expect('}');
                return m;
            }
            if (take('[')) {
                List<Object> l = new ArrayList<>();
                if (take(']')) {
                    return l;
                }
                do {
                    l.add(value());
                } while (take(','));
                expect(']');
                return l;
            }
            ws();
            return string();
        }

        private String string() {
            if (i >= s.length() || s.charAt(i) != '"') {
                throw fail("want a string");
            }
            i++;
            StringBuilder b = new StringBuilder();
            while (i < s.length() && s.charAt(i) != '"') {
                char c = s.charAt(i++);
                if (c == '\\') {
                    if (i >= s.length()) {
                        throw fail("escape at the end");
                    }
                    char e = s.charAt(i++);
                    if (e != '"' && e != '\\' && e != '/') {
                        throw fail("escape \\" + e);
                    }
                    c = e;
                }
                b.append(c);
            }
            if (i >= s.length()) {
                throw fail("unterminated string");
            }
            i++;
            return b.toString();
        }
    }
}

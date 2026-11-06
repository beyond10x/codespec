package codespec.read.groovy;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * JSON for cir/1 documents: objects are {@link LinkedHashMap}s in insertion
 * order, arrays {@link List}s, and scalars {@link String} and {@link Boolean}. {@link #write} prints
 * the layout {@code serde_json::to_string_pretty} prints (two-space indentation, {@code ": "},
 * {@code []} and {@code {}} when empty, a final newline), so a document written in the model's field
 * order is already canonical.
 *
 * <p>A copy of the Java reader's {@code codespec.read.Json}, byte for byte the same apart from
 * the package and this comment.
 */
public final class Json {
    private Json() {}

    /** A new, empty object. */
    public static Map<String, Object> obj() {
        return new LinkedHashMap<>();
    }

    public static String write(Object v) {
        StringBuilder b = new StringBuilder();
        value(b, v, 0);
        b.append('\n');
        return b.toString();
    }

    private static void indent(StringBuilder b, int n) {
        for (int i = 0; i < n; i++) {
            b.append("  ");
        }
    }

    @SuppressWarnings("unchecked")
    private static void value(StringBuilder b, Object v, int depth) {
        if (v instanceof String) {
            string(b, (String) v);
        } else if (v instanceof Boolean) {
            b.append(v.toString());
        } else if (v instanceof Map) {
            Map<String, Object> m = (Map<String, Object>) v;
            if (m.isEmpty()) {
                b.append("{}");
                return;
            }
            b.append("{\n");
            int i = 0;
            for (Map.Entry<String, Object> e : m.entrySet()) {
                indent(b, depth + 1);
                string(b, e.getKey());
                b.append(": ");
                value(b, e.getValue(), depth + 1);
                b.append(++i < m.size() ? ",\n" : "\n");
            }
            indent(b, depth);
            b.append('}');
        } else if (v instanceof List) {
            List<Object> l = (List<Object>) v;
            if (l.isEmpty()) {
                b.append("[]");
                return;
            }
            b.append("[\n");
            for (int i = 0; i < l.size(); i++) {
                indent(b, depth + 1);
                value(b, l.get(i), depth + 1);
                b.append(i + 1 < l.size() ? ",\n" : "\n");
            }
            indent(b, depth);
            b.append(']');
        } else {
            throw new IllegalArgumentException("no JSON form for " + v);
        }
    }

    /** A string as serde_json escapes it: quote, backslash and control characters only. */
    private static void string(StringBuilder b, String s) {
        b.append('"');
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"':
                    b.append("\\\"");
                    break;
                case '\\':
                    b.append("\\\\");
                    break;
                case '\n':
                    b.append("\\n");
                    break;
                case '\r':
                    b.append("\\r");
                    break;
                case '\t':
                    b.append("\\t");
                    break;
                case '\b':
                    b.append("\\b");
                    break;
                case '\f':
                    b.append("\\f");
                    break;
                default:
                    if (c < 0x20) {
                        b.append(String.format("\\u%04x", (int) c));
                    } else {
                        b.append(c);
                    }
            }
        }
        b.append('"');
    }

    /** Parse a document: objects, arrays, strings, booleans and null; numbers are refused. */
    public static Object parse(String s) {
        Parser p = new Parser(s);
        Object v = p.value();
        p.ws();
        if (p.i != s.length()) {
            throw p.fail("trailing input");
        }
        return v;
    }

    private static final class Parser {
        final String s;
        int i;

        Parser(String s) {
            this.s = s;
        }

        IllegalArgumentException fail(String what) {
            return new IllegalArgumentException("JSON at " + i + ": " + what);
        }

        void ws() {
            while (i < s.length() && Character.isWhitespace(s.charAt(i))) {
                i++;
            }
        }

        void expect(char c) {
            ws();
            if (i >= s.length() || s.charAt(i) != c) {
                throw fail("want " + c);
            }
            i++;
        }

        boolean peek(char c) {
            ws();
            return i < s.length() && s.charAt(i) == c;
        }

        Object value() {
            ws();
            if (i >= s.length()) {
                throw fail("end of input");
            }
            char c = s.charAt(i);
            if (c == '{') {
                i++;
                Map<String, Object> m = obj();
                if (peek('}')) {
                    i++;
                    return m;
                }
                do {
                    ws();
                    String k = str();
                    expect(':');
                    m.put(k, value());
                } while (comma());
                expect('}');
                return m;
            }
            if (c == '[') {
                i++;
                List<Object> l = new ArrayList<>();
                if (peek(']')) {
                    i++;
                    return l;
                }
                do {
                    l.add(value());
                } while (comma());
                expect(']');
                return l;
            }
            if (c == '"') {
                return str();
            }
            for (String w : new String[] {"true", "false", "null"}) {
                if (s.startsWith(w, i)) {
                    i += w.length();
                    return w.equals("null") ? null : Boolean.valueOf(w.equals("true"));
                }
            }
            throw fail("unexpected " + c);
        }

        boolean comma() {
            if (peek(',')) {
                i++;
                return true;
            }
            return false;
        }

        String str() {
            if (i >= s.length() || s.charAt(i) != '"') {
                throw fail("want a string");
            }
            i++;
            StringBuilder b = new StringBuilder();
            while (true) {
                if (i >= s.length()) {
                    throw fail("unterminated string");
                }
                char c = s.charAt(i++);
                if (c == '"') {
                    return b.toString();
                }
                if (c != '\\') {
                    b.append(c);
                    continue;
                }
                char e = s.charAt(i++);
                switch (e) {
                    case 'n':
                        b.append('\n');
                        break;
                    case 'r':
                        b.append('\r');
                        break;
                    case 't':
                        b.append('\t');
                        break;
                    case 'b':
                        b.append('\b');
                        break;
                    case 'f':
                        b.append('\f');
                        break;
                    case 'u':
                        b.append((char) Integer.parseInt(s.substring(i, i + 4), 16));
                        i += 4;
                        break;
                    default:
                        b.append(e);
                }
            }
        }
    }
}

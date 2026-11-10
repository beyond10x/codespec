package codespec.run;

import java.io.IOException;
import java.io.PrintStream;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Stream;

/**
 * The JVM runner of the conformance kit ({@code docs/conformance.md} § 4), shared by {@code
 * codespec-run-java} and {@code codespec-run-groovy}: it compiles the source files of one package
 * directory, makes the calls of a {@code codespec-calls/1} document against them and prints one line
 * per call in the harness encoding the crate doc of {@code codespec_conformance} defines.
 *
 * <pre>
 * codespec-run-&lt;lang&gt; [--build &lt;dir&gt;] &lt;package dir&gt; &lt;calls.json&gt;
 * </pre>
 *
 * <p>A call's {@code func} is a Go name ({@code Below}); it calls the one {@code public static}
 * method of a class of the package whose name, its first letter upper-cased, is that name, as the
 * JVM readers export a method. No such method, or more than one, is refused naming the {@code func}.
 *
 * <table>
 *   <caption>Parameters</caption>
 *   <tr><th>parameter</th><th>argument</th></tr>
 *   <tr><td>{@code long}, {@code int}, {@code short}, {@code byte} and their boxes</td><td>the
 *       integer, refused when it does not fit the type</td></tr>
 *   <tr><td>{@code boolean}, {@code Boolean}</td><td>{@code true}, {@code false}</td></tr>
 *   <tr><td>{@code String}</td><td>the string between the quotes</td></tr>
 * </table>
 *
 * <table>
 *   <caption>Results</caption>
 *   <tr><th>result</th><th>line</th></tr>
 *   <tr><td>{@code void}</td><td>{@code ok}</td></tr>
 *   <tr><td>a value</td><td>{@code ok <v>}</td></tr>
 *   <tr><td>an exception whose class the package declares</td><td>{@code err <getMessage()>}</td></tr>
 * </table>
 *
 * <p>A printable value is an integer in decimal, {@code true} or {@code false}, a {@code String} as
 * Go's {@code strconv.Quote} writes it, a {@code List} as {@code [} its elements separated by one
 * space {@code ]} and an enum constant as its name, quoted, since the JVM readers read an enum over
 * {@code string}. A result of type {@code Object} prints the value it holds, so a {@code null} there
 * is refused when the call returns it. A string holding a character outside printable ASCII other than the escapes
 * {@code strconv.Quote} writes the same way for every character is refused (exit 2), and so is any
 * other parameter or result type.
 *
 * <p>The build directory ({@code --build}, emptied first; else a directory under the system's
 * temporary directory, removed afterwards) holds the compiled classes under {@code classes/}.
 *
 * <table>
 *   <caption>Exit status</caption>
 *   <tr><td>0</td><td>every call made and printed</td></tr>
 *   <tr><td>2</td><td>refused: {@code calls.json} not as {@code read_calls} reads it, a call that
 *       does not fit the package, a value the runner cannot print exactly; a usage error too</td></tr>
 *   <tr><td>1</td><td>failed: the package does not compile, or the program fails with an exception
 *       the package does not declare</td></tr>
 * </table>
 */
public final class Runner {
    /** Compiles the package's source files into a class directory. */
    public interface Compiler {
        /** The extension of the files compiled, {@code .java} or {@code .groovy}. */
        String extension();

        /** Compile {@code sources} into {@code classes}; a failure throws {@link Failed}. */
        void compile(List<Path> sources, Path classes) throws Failed;
    }

    /** The input does not fit: exit 2. */
    public static final class Refused extends Exception {
        private static final long serialVersionUID = 1L;

        public Refused(String message) {
            super(message);
        }
    }

    /** The build or the program failed: exit 1. */
    public static final class Failed extends Exception {
        private static final long serialVersionUID = 1L;

        public Failed(String message) {
            super(message);
        }
    }

    /** One call of {@code calls.json}. */
    record Call(String func, List<String> args) {}

    private Runner() {}

    /** The command line of {@code codespec-run-<name>}: parse, run, print, exit. */
    public static void main(String name, Compiler compiler, String[] argv) {
        Path build = null;
        List<String> rest = new ArrayList<>();
        for (int i = 0; i < argv.length; i++) {
            if ((argv[i].equals("--build") || argv[i].equals("-build")) && i + 1 < argv.length) {
                build = Path.of(argv[++i]);
            } else {
                rest.add(argv[i]);
            }
        }
        if (rest.size() != 2) {
            System.err.println("usage: " + name + " [--build <dir>] <package dir> <calls.json>");
            System.exit(2);
        }
        PrintStream out = new PrintStream(System.out, false, StandardCharsets.UTF_8);
        int code = 0;
        Path tmp = null;
        try {
            List<Call> calls = readCalls(Path.of(rest.get(1)));
            if (build == null) {
                tmp = Files.createTempDirectory(name + "-");
                build = tmp;
            }
            run(compiler, Path.of(rest.get(0)), calls, build, out);
        } catch (Refused e) {
            System.err.println(name + ": refused: " + e.getMessage());
            code = 2;
        } catch (Failed e) {
            System.err.println(name + ": " + e.getMessage());
            code = 1;
        } catch (IOException e) {
            System.err.println(name + ": " + e);
            code = 1;
        } finally {
            out.flush();
            if (tmp != null) {
                delete(tmp);
            }
        }
        System.exit(code);
    }

    /** Compile the package in {@code dir} into {@code build/classes} and make every call. */
    public static void run(Compiler compiler, Path dir, List<Call> calls, Path build, PrintStream out)
            throws Refused, Failed, IOException {
        List<Path> sources;
        try (Stream<Path> s = Files.list(dir)) {
            sources = s.filter(p -> p.getFileName().toString().endsWith(compiler.extension()))
                    .sorted()
                    .toList();
        }
        if (sources.isEmpty()) {
            throw new Failed(dir + " holds no " + compiler.extension() + " file");
        }
        delete(build);
        Path classes = build.resolve("classes");
        Files.createDirectories(classes);
        compiler.compile(sources, classes);
        List<String> names = new ArrayList<>();
        try (Stream<Path> s = Files.walk(classes)) {
            for (Path p : s.filter(p -> p.toString().endsWith(".class")).sorted().toList()) {
                String rel = classes.relativize(p).toString();
                names.add(rel.substring(0, rel.length() - ".class".length()).replace('/', '.'));
            }
        }
        try (URLClassLoader loader =
                new URLClassLoader(new URL[] {classes.toUri().toURL()}, Runner.class.getClassLoader())) {
            List<Class<?>> loaded = new ArrayList<>();
            for (String n : names) {
                try {
                    loaded.add(Class.forName(n, false, loader));
                } catch (ClassNotFoundException | LinkageError e) {
                    throw new Failed("loading " + n + ": " + e);
                }
            }
            List<Method> resolved = new ArrayList<>();
            for (Call c : calls) {
                Method m = find(loaded, c.func());
                resolved.add(m);
                if (m.getParameterCount() != c.args().size()) {
                    throw new Refused(c.func() + " takes " + m.getParameterCount() + " arguments, the call gives "
                            + c.args().size());
                }
                for (int i = 0; i < c.args().size(); i++) {
                    argument(m.getParameterTypes()[i], c.args().get(i), c.func());
                }
                printable(m.getReturnType(), c.func());
            }
            PrintStream saved = System.out;
            System.setOut(out);
            try {
                for (int i = 0; i < calls.size(); i++) {
                    Call c = calls.get(i);
                    Method m = resolved.get(i);
                    Object[] args = new Object[c.args().size()];
                    for (int j = 0; j < args.length; j++) {
                        args[j] = argument(m.getParameterTypes()[j], c.args().get(j), c.func());
                    }
                    String line;
                    try {
                        Object v = m.invoke(null, args);
                        line = m.getReturnType() == void.class ? "ok" : "ok " + value(v);
                    } catch (InvocationTargetException e) {
                        Throwable t = e.getCause();
                        if (t != null && t.getClass().getClassLoader() == loader && t instanceof Exception) {
                            line = "err " + t.getMessage();
                        } else {
                            throw new Failed(c.func() + " threw " + t);
                        }
                    } catch (IllegalAccessException e) {
                        throw new Failed(c.func() + ": " + e);
                    }
                    out.println(line);
                }
            } finally {
                out.flush();
                System.setOut(saved);
            }
        }
    }

    /** The one public static method of the package exported as {@code func}. */
    static Method find(List<Class<?>> classes, String func) throws Refused {
        List<Method> found = new ArrayList<>();
        for (Class<?> k : classes) {
            for (Method m : k.getDeclaredMethods()) {
                int mod = m.getModifiers();
                if (Modifier.isPublic(mod) && Modifier.isStatic(mod) && !m.isSynthetic()
                        && exported(m.getName()).equals(func)) {
                    found.add(m);
                }
            }
        }
        if (found.isEmpty()) {
            throw new Refused("no public static method exported as " + func);
        }
        if (found.size() > 1) {
            throw new Refused(found.size() + " public static methods exported as " + func);
        }
        return found.get(0);
    }

    static String exported(String name) {
        return name.isEmpty() ? name : Character.toUpperCase(name.charAt(0)) + name.substring(1);
    }

    /** {@code arg} as a value of the parameter type {@code t}, or refused. */
    static Object argument(Class<?> t, String arg, String func) throws Refused {
        String why = func + ": argument " + arg + " is not a " + t.getSimpleName() + " value";
        if (t == long.class || t == Long.class || t == int.class || t == Integer.class || t == short.class
                || t == Short.class || t == byte.class || t == Byte.class) {
            String digits = arg.startsWith("-") ? arg.substring(1) : arg;
            if (digits.isEmpty() || !digits.chars().allMatch(c -> c >= '0' && c <= '9')
                    || (digits.length() > 1 && digits.startsWith("0")) || arg.equals("-0")) {
                throw new Refused(why);
            }
            long v;
            try {
                v = Long.parseLong(arg);
            } catch (NumberFormatException e) {
                throw new Refused(why);
            }
            if (t == long.class || t == Long.class) {
                return v;
            }
            if (t == int.class || t == Integer.class) {
                if (v != (int) v) {
                    throw new Refused(why);
                }
                return (int) v;
            }
            if (t == short.class || t == Short.class) {
                if (v != (short) v) {
                    throw new Refused(why);
                }
                return (short) v;
            }
            if (v != (byte) v) {
                throw new Refused(why);
            }
            return (byte) v;
        }
        if (t == boolean.class || t == Boolean.class) {
            if (arg.equals("true") || arg.equals("false")) {
                return Boolean.valueOf(arg);
            }
            throw new Refused(why);
        }
        if (t == String.class) {
            if (arg.length() < 2 || !arg.startsWith("\"") || !arg.endsWith("\"")) {
                throw new Refused(why);
            }
            String s = arg.substring(1, arg.length() - 1);
            if (!s.chars().allMatch(c -> c >= ' ' && c <= '~' && c != '"' && c != '\\')) {
                throw new Refused(why);
            }
            return s;
        }
        throw new Refused(func + ": parameter type " + t.getName() + " cannot be passed");
    }

    /** Refuse a result type the runner cannot print, before anything runs. */
    static void printable(Class<?> t, String func) throws Refused {
        if (t == void.class || t == long.class || t == Long.class || t == int.class || t == Integer.class
                || t == short.class || t == Short.class || t == byte.class || t == Byte.class
                || t == boolean.class || t == Boolean.class || t == String.class || t.isEnum()
                || List.class.isAssignableFrom(t) || t == Object.class) {
            return;
        }
        throw new Refused(func + ": result type " + t.getName() + " cannot be printed");
    }

    /** One value in the harness encoding. */
    static String value(Object v) throws Refused {
        if (v instanceof Long || v instanceof Integer || v instanceof Short || v instanceof Byte
                || v instanceof Boolean) {
            return v.toString();
        }
        if (v instanceof String s) {
            return quote(s);
        }
        if (v instanceof Enum<?> e) {
            return quote(e.name());
        }
        if (v instanceof List<?> l) {
            StringBuilder b = new StringBuilder("[");
            for (int i = 0; i < l.size(); i++) {
                if (i > 0) {
                    b.append(' ');
                }
                b.append(value(l.get(i)));
            }
            return b.append(']').toString();
        }
        throw new Refused("a value of " + (v == null ? "null" : v.getClass().getName()) + " cannot be printed");
    }

    /** {@code s} as Go's {@code strconv.Quote} writes it; a character outside ASCII is refused. */
    static String quote(String s) throws Refused {
        StringBuilder b = new StringBuilder("\"");
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"' -> b.append("\\\"");
                case '\\' -> b.append("\\\\");
                case 7 -> b.append("\\a");
                case '\b' -> b.append("\\b");
                case '\f' -> b.append("\\f");
                case '\n' -> b.append("\\n");
                case '\r' -> b.append("\\r");
                case '\t' -> b.append("\\t");
                case 11 -> b.append("\\v");
                default -> {
                    if (c > 0x7e) {
                        throw new Refused("a string holding a character outside ASCII cannot be printed exactly");
                    }
                    if (c < ' ' || c == 0x7f) {
                        b.append(String.format("\\x%02x", (int) c));
                    } else {
                        b.append(c);
                    }
                }
            }
        }
        return b.append('"').toString();
    }

    // ---- calls.json ----

    /**
     * Read {@code calls.json} as {@code read_calls} does: the format {@code codespec-calls/1}, a
     * non-empty list of calls each with exactly {@code func} and {@code args}, in the canonical
     * encoding (two-space indentation, a final newline).
     */
    static List<Call> readCalls(Path path) throws Refused, IOException {
        String text = Files.readString(path, StandardCharsets.UTF_8);
        Object doc;
        try {
            Parser p = new Parser(text);
            doc = p.value();
            p.end();
        } catch (IllegalArgumentException e) {
            throw new Refused(path + ": " + e.getMessage());
        }
        if (!(doc instanceof Map<?, ?> top) || !top.keySet().equals(java.util.Set.of("format", "calls"))) {
            throw new Refused(path + ": a codespec-calls/1 document holds exactly format and calls");
        }
        if (!"codespec-calls/1".equals(top.get("format"))) {
            throw new Refused(path + ": format " + top.get("format") + ", not codespec-calls/1");
        }
        if (!(top.get("calls") instanceof List<?> list) || list.isEmpty()) {
            throw new Refused(path + ": no calls");
        }
        List<Call> calls = new ArrayList<>();
        for (Object o : list) {
            if (!(o instanceof Map<?, ?> c) || !c.keySet().equals(java.util.Set.of("func", "args"))
                    || !(c.get("func") instanceof String f) || !(c.get("args") instanceof List<?> a)) {
                throw new Refused(path + ": a call holds exactly a string func and a list args");
            }
            List<String> args = new ArrayList<>();
            for (Object x : a) {
                if (!(x instanceof String s)) {
                    throw new Refused(path + ": an argument of " + f + " is not a string");
                }
                args.add(s);
            }
            calls.add(new Call(f, args));
        }
        if (!canonical(calls).equals(text)) {
            throw new Refused(path + " is not canonical");
        }
        return calls;
    }

    /** The canonical encoding of a {@code codespec-calls/1} document holding {@code calls}. */
    static String canonical(List<Call> calls) {
        StringBuilder b = new StringBuilder("{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n");
        for (int i = 0; i < calls.size(); i++) {
            Call c = calls.get(i);
            b.append("    {\n      \"func\": ").append(json(c.func())).append(",\n      \"args\": [");
            if (c.args().isEmpty()) {
                b.append("]\n");
            } else {
                b.append('\n');
                for (int j = 0; j < c.args().size(); j++) {
                    b.append("        ").append(json(c.args().get(j)));
                    b.append(j < c.args().size() - 1 ? ",\n" : "\n");
                }
                b.append("      ]\n");
            }
            b.append(i < calls.size() - 1 ? "    },\n" : "    }\n");
        }
        return b.append("  ]\n}\n").toString();
    }

    /** {@code s} as serde_json writes a JSON string. */
    static String json(String s) {
        StringBuilder b = new StringBuilder("\"");
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"' -> b.append("\\\"");
                case '\\' -> b.append("\\\\");
                case '\b' -> b.append("\\b");
                case '\f' -> b.append("\\f");
                case '\n' -> b.append("\\n");
                case '\r' -> b.append("\\r");
                case '\t' -> b.append("\\t");
                default -> {
                    if (c < ' ') {
                        b.append(String.format("\\u%04x", (int) c));
                    } else {
                        b.append(c);
                    }
                }
            }
        }
        return b.append('"').toString();
    }

    /** A strict JSON parser of objects, arrays and strings, all a {@code calls.json} holds. */
    static final class Parser {
        private final String s;
        private int i;

        Parser(String s) {
            this.s = s;
        }

        void end() {
            ws();
            if (i != s.length()) {
                throw new IllegalArgumentException("trailing text at offset " + i);
            }
        }

        private void ws() {
            while (i < s.length() && " \t\n\r".indexOf(s.charAt(i)) >= 0) {
                i++;
            }
        }

        private void expect(char c) {
            ws();
            if (i >= s.length() || s.charAt(i) != c) {
                throw new IllegalArgumentException("expected '" + c + "' at offset " + i);
            }
            i++;
        }

        private boolean peek(char c) {
            ws();
            return i < s.length() && s.charAt(i) == c;
        }

        Object value() {
            ws();
            if (peek('{')) {
                i++;
                Map<String, Object> m = new LinkedHashMap<>();
                if (peek('}')) {
                    i++;
                    return m;
                }
                do {
                    ws();
                    String k = string();
                    expect(':');
                    if (m.put(k, value()) != null) {
                        throw new IllegalArgumentException("duplicate key " + k);
                    }
                } while (comma());
                expect('}');
                return m;
            }
            if (peek('[')) {
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
            if (peek('"')) {
                return string();
            }
            throw new IllegalArgumentException("expected an object, array or string at offset " + i);
        }

        private boolean comma() {
            if (peek(',')) {
                i++;
                return true;
            }
            return false;
        }

        private String string() {
            expect('"');
            StringBuilder b = new StringBuilder();
            while (true) {
                if (i >= s.length()) {
                    throw new IllegalArgumentException("unterminated string");
                }
                char c = s.charAt(i++);
                if (c == '"') {
                    return b.toString();
                }
                if (c < ' ') {
                    throw new IllegalArgumentException("control character in a string at offset " + (i - 1));
                }
                if (c != '\\') {
                    b.append(c);
                    continue;
                }
                if (i >= s.length()) {
                    throw new IllegalArgumentException("unterminated escape");
                }
                char e = s.charAt(i++);
                switch (e) {
                    case '"' -> b.append('"');
                    case '\\' -> b.append('\\');
                    case '/' -> b.append('/');
                    case 'b' -> b.append('\b');
                    case 'f' -> b.append('\f');
                    case 'n' -> b.append('\n');
                    case 'r' -> b.append('\r');
                    case 't' -> b.append('\t');
                    case 'u' -> {
                        if (i + 4 > s.length()) {
                            throw new IllegalArgumentException("short \\u escape");
                        }
                        b.append((char) Integer.parseInt(s.substring(i, i + 4), 16));
                        i += 4;
                    }
                    default -> throw new IllegalArgumentException("bad escape \\" + e);
                }
            }
        }
    }

    /** Delete {@code dir} and everything under it, if it exists. */
    static void delete(Path dir) {
        if (!Files.exists(dir)) {
            return;
        }
        try (Stream<Path> s = Files.walk(dir)) {
            for (Path p : s.sorted(Comparator.reverseOrder()).toList()) {
                Files.delete(p);
            }
        } catch (IOException e) {
            throw new java.io.UncheckedIOException(e);
        }
    }
}

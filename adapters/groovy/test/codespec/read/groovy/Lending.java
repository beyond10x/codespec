package codespec.read.groovy;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.TreeSet;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * What a read of {@code testdata/lending} must give, per artefact, from the fixture's own two
 * sources: the {@code // expect:} markers in each file and the "Per artefact" section of {@code
 * EXPECTED.md}. The grammar is in {@code EXPECTED.md}; {@code crates/cli/tests/fixture.rs} checks
 * the two agree with each other. {@link #compare} checks a read module against both and returns
 * every mismatch, each naming the file and, for a marker, its line.
 */
final class Lending {
    /** The functions read and refused and the refusals by category of one artefact. */
    static final class Artefact {
        final TreeSet<String> read = new TreeSet<>();
        final TreeSet<String> refused = new TreeSet<>();
        final TreeMap<String, Integer> categories = new TreeMap<>();

        @Override
        public String toString() {
            return "read " + read + ", refused " + refused + ", refusals " + categories;
        }

        @Override
        public boolean equals(Object o) {
            return o instanceof Artefact && toString().equals(o.toString());
        }

        @Override
        public int hashCode() {
            return toString().hashCode();
        }
    }

    private Lending() {}

    private static final Pattern SECTION = Pattern.compile("^### `([^`]+)`$");

    /** The "Per artefact" section of {@code EXPECTED.md}, by path relative to the application. */
    static Map<String, Artefact> expected(Path app) throws IOException {
        Map<String, Artefact> out = new TreeMap<>();
        Artefact cur = null;
        boolean inPer = false;
        for (String line : Files.readAllLines(app.resolve("EXPECTED.md"))) {
            if (line.startsWith("## ")) {
                inPer = line.equals("## Per artefact");
                cur = null;
                continue;
            }
            if (!inPer) {
                continue;
            }
            Matcher m = SECTION.matcher(line);
            if (m.matches()) {
                cur = new Artefact();
                out.put(m.group(1), cur);
            } else if (cur != null && line.startsWith("- read: ")) {
                cur.read.addAll(names(line.substring("- read: ".length())));
            } else if (cur != null && line.startsWith("- refused: ")) {
                cur.refused.addAll(names(line.substring("- refused: ".length())));
            } else if (cur != null && line.startsWith("- refusals: ")) {
                String cell = line.substring("- refusals: ".length()).trim();
                if (!cell.equals("none")) {
                    for (String part : cell.split(",")) {
                        String[] cn = part.trim().split(" ");
                        cur.categories.put(cn[0], Integer.parseInt(cn[1]));
                    }
                }
            }
        }
        return out;
    }

    private static List<String> names(String cell) {
        List<String> out = new ArrayList<>();
        if (cell.trim().equals("none")) {
            return out;
        }
        for (String n : cell.split(",")) {
            out.add(n.trim().replace("`", ""));
        }
        return out;
    }

    /** What the module gives for one artefact: its functions, read or refused, and its refusals by category. */
    static Artefact actual(Map<String, Object> doc, Path app, String file) throws IOException {
        Artefact a = new Artefact();
        List<String> lines = Files.readAllLines(app.resolve(file));
        for (Object f : list(doc.get("funcs"))) {
            Map<String, Object> fn = map(f);
            if (!file.equals(fileOf((String) fn.get("pos"))) || synthesized(fn, lines)) {
                continue;
            }
            (hasForeign(fn) ? a.refused : a.read).add((String) fn.get("name"));
        }
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            String pos = (String) rm.get("pos");
            if (!file.equals(fileOf(pos))) {
                continue;
            }
            String reason = (String) rm.get("reason");
            a.categories.merge(reason.substring(0, reason.indexOf(": ")), 1, Integer::sum);
            String method = declaredMethod((String) rm.get("what"), lines, lineOf(pos));
            if (method != null) {
                a.refused.add(method);
            }
        }
        return a;
    }

    /**
     * Every mismatch between the module and the fixture: per marker, at its file and line, and per
     * artefact, against {@code EXPECTED.md}. Empty when the read matches.
     */
    static List<String> compare(Map<String, Object> doc, Path app) throws IOException {
        List<String> problems = new ArrayList<>();
        Map<String, Artefact> expected = expected(app);
        for (Map.Entry<String, Artefact> e : expected.entrySet()) {
            Artefact got = actual(doc, app, e.getKey());
            if (!got.equals(e.getValue())) {
                problems.add(e.getKey() + ": EXPECTED.md says " + e.getValue() + "; the read gives " + got);
            }
            problems.addAll(markers(doc, app, e.getKey()));
        }
        for (Object r : list(doc.get("refusals"))) {
            String f = fileOf((String) map(r).get("pos"));
            if (!expected.containsKey(f)) {
                problems.add(map(r).get("pos") + ": a refusal in a file EXPECTED.md does not list: " + map(r).get("reason"));
            }
        }
        for (Object f : list(doc.get("funcs"))) {
            String file = fileOf((String) map(f).get("pos"));
            if (!expected.containsKey(file)) {
                problems.add(map(f).get("pos") + ": function " + map(f).get("name") + " in a file EXPECTED.md does not list");
            }
        }
        return problems;
    }

    private static final Pattern MARK = Pattern.compile("// expect: (read|refused)(?: ([a-z-]+))?\\s*$");

    /** Each marker of one artefact against the refusals and functions at its line. */
    private static List<String> markers(Map<String, Object> doc, Path app, String file) throws IOException {
        List<String> problems = new ArrayList<>();
        List<String> lines = Files.readAllLines(app.resolve(file));
        boolean whole = false;
        for (String l : lines) {
            if (l.trim().startsWith("import ") && l.contains("// expect: refused unresolved")) {
                whole = true;
            }
        }
        Map<Integer, List<String>> refusedAt = new TreeMap<>();
        for (Object r : list(doc.get("refusals"))) {
            Map<String, Object> rm = map(r);
            String pos = (String) rm.get("pos");
            if (file.equals(fileOf(pos))) {
                String reason = (String) rm.get("reason");
                refusedAt.computeIfAbsent(lineOf(pos), k -> new ArrayList<>()).add(reason.substring(0, reason.indexOf(": ")));
            }
        }
        Map<Integer, String> marked = new TreeMap<>();
        for (int i = 0; i < lines.size(); i++) {
            Matcher m = MARK.matcher(lines.get(i));
            if (!m.find()) {
                continue;
            }
            int n = i + 1;
            boolean importLine = lines.get(i).trim().startsWith("import ");
            if (whole && !importLine) {
                continue;
            }
            String at = file + ":" + n;
            if (m.group(1).equals("refused") && m.group(2) != null) {
                marked.put(n, m.group(2));
                List<String> got = refusedAt.getOrDefault(n, List.of());
                if (!got.equals(List.of(m.group(2)))) {
                    problems.add(at + ": marked refused " + m.group(2) + "; the read refuses here " + got);
                }
            } else if (m.group(1).equals("read")) {
                String method = methodName(lines.get(i));
                if (method != null) {
                    Map<String, Object> fn = funcAt(doc, at);
                    if (fn == null || !method.equals(fn.get("name")) || hasForeign(fn)) {
                        problems.add(at + ": " + method + " is marked read; the read gives "
                            + (fn == null ? "no function here" : fn.get("name") + (hasForeign(fn) ? " with a foreign statement" : "")));
                    }
                } else if (refusedAt.containsKey(n)) {
                    problems.add(at + ": marked read; the read refuses here " + refusedAt.get(n));
                } else if (statementLine(lines.get(i))) {
                    if (!readStatementAt(doc.get("funcs"), at)) {
                        problems.add(at + ": a statement marked read; the read has no statement here outside foreign");
                    }
                } else if (!declared(doc, lines.get(i))) {
                    problems.add(at + ": marked read; the read has no field or constant " + memberName(lines.get(i)));
                }
            }
        }
        for (Map.Entry<Integer, List<String>> e : refusedAt.entrySet()) {
            if (!marked.containsKey(e.getKey())) {
                problems.add(file + ":" + e.getKey() + ": the read refuses here " + e.getValue() + "; the line carries no refused marker");
            }
        }
        return problems;
    }

    private static final Pattern METHOD = Pattern.compile("^\\s*(?:[\\w<>,\\[\\] ]+\\s)?(\\w+)\\s*\\([^)]*\\)\\s*\\{");

    /** The name a method declaration line declares, or null for any other line. */
    static String methodName(String line) {
        String code = line.contains("//") ? line.substring(0, line.indexOf("//")) : line;
        if (code.contains("=") || code.trim().startsWith("return ")) {
            return null;
        }
        Matcher m = METHOD.matcher(code);
        if (!m.find()) {
            return null;
        }
        String name = m.group(1);
        return List.of("if", "for", "while", "switch", "catch").contains(name) ? null : name;
    }

    /** Whether a line opens a statement with a block ({@code for}, {@code while}, {@code switch}, {@code if}). */
    private static boolean statementLine(String line) {
        String code = (line.contains("//") ? line.substring(0, line.indexOf("//")) : line).trim();
        return code.endsWith("{") && code.matches("^(for|while|switch|if)\\b.*");
    }

    /** Whether a function of the module holds a statement at {@code pos} that is not foreign. */
    private static boolean readStatementAt(Object o, String pos) {
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if (pos.equals(m.get("pos")) && m.containsKey("k") && !"foreign".equals(m.get("k")) && !m.containsKey("params")) {
                return true;
            }
            for (Object v : m.values()) {
                if (readStatementAt(v, pos)) {
                    return true;
                }
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                if (readStatementAt(v, pos)) {
                    return true;
                }
            }
        }
        return false;
    }

    /** The name a property or constant line declares: the word before {@code =}, else the last word. */
    private static String memberName(String line) {
        String code = line.contains("//") ? line.substring(0, line.indexOf("//")) : line;
        if (code.contains("=")) {
            code = code.substring(0, code.indexOf('='));
        }
        String[] words = code.trim().split("\\s+");
        return words[words.length - 1];
    }

    /** Whether the module holds a struct field or a const of the name a line declares. */
    private static boolean declared(Map<String, Object> doc, String line) {
        String name = memberName(line);
        for (Object c : list(doc.get("consts"))) {
            if (name.equals(map(c).get("name"))) {
                return true;
            }
        }
        for (Object t : list(doc.get("types"))) {
            for (Object f : list(map(t).get("fields"))) {
                if (name.equals(map(f).get("name"))) {
                    return true;
                }
            }
        }
        return false;
    }

    /** The method a declaration refusal names, {@code Class.method(…)}, when the refused line declares it. */
    private static String declaredMethod(String what, List<String> lines, int line) {
        if (!what.endsWith("(…)")) {
            return null;
        }
        what = what.substring(0, what.length() - "(…)".length());
        int dot = what.lastIndexOf('.');
        if (dot < 0 || line < 1 || line > lines.size()) {
            return null;
        }
        String name = what.substring(dot + 1);
        return name.equals(methodName(lines.get(line - 1))) ? name : null;
    }

    /** A constructor the reader made for a class without one: {@code New<Class>} at the class line. */
    private static boolean synthesized(Map<String, Object> fn, List<String> lines) {
        String name = (String) fn.get("name");
        int line = lineOf((String) fn.get("pos"));
        return name.startsWith("New") && line >= 1 && line <= lines.size() && lines.get(line - 1).contains("class " + name.substring(3));
    }

    private static Map<String, Object> funcAt(Map<String, Object> doc, String pos) {
        for (Object f : list(doc.get("funcs"))) {
            if (pos.equals(map(f).get("pos"))) {
                return map(f);
            }
        }
        return null;
    }

    static boolean hasForeign(Object o) {
        if (o instanceof Map) {
            Map<String, Object> m = map(o);
            if ("foreign".equals(m.get("k"))) {
                return true;
            }
            for (Object v : m.values()) {
                if (hasForeign(v)) {
                    return true;
                }
            }
        } else if (o instanceof List) {
            for (Object v : list(o)) {
                if (hasForeign(v)) {
                    return true;
                }
            }
        }
        return false;
    }

    static String fileOf(String pos) {
        return pos.substring(0, pos.lastIndexOf(':'));
    }

    static int lineOf(String pos) {
        return Integer.parseInt(pos.substring(pos.lastIndexOf(':') + 1));
    }

    @SuppressWarnings("unchecked")
    static Map<String, Object> map(Object o) {
        if (!(o instanceof Map)) {
            throw new AssertionError("not an object: " + o);
        }
        return (Map<String, Object>) o;
    }

    @SuppressWarnings("unchecked")
    static List<Object> list(Object o) {
        if (o == null) {
            return List.of();
        }
        if (!(o instanceof List)) {
            throw new AssertionError("not an array: " + o);
        }
        return (List<Object>) o;
    }
}

package codespec.read;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * A permitted record that declares a member is no cir/1 union variant, however its body's source
 * ends. A line comment ending in an opening brace as the body's last line makes a record that
 * declares a method, or a compact constructor that validates, end in an empty pair of braces once
 * whitespace is removed; the reader decides from the record's members, not its text, so the union is
 * refused at Square's line as {@code testdata/refuse/union-member} requires for the plain form. The
 * same holds for every other kind of member a record can declare: an accessor, a canonical
 * constructor, a static field or method, a nested type, an initializer.
 *
 * <p>Usage: {@code java -cp <classes> codespec.read.UnionMemberTest <work dir>}. Exit 1 when red.
 */
public final class UnionMemberTest {
    private UnionMemberTest() {}

    public static void main(String[] args) throws Exception {
        Path work = Path.of(args[0]);
        List<String> failed = new ArrayList<>();
        check(failed, work, "method", String.join("\n",
            "package union_method_comment;",
            "",
            "public final class Shapes {",
            "    public static long one() {",
            "        return 1L;",
            "    }",
            "}",
            "",
            "sealed interface Shape permits Circle, Square {}",
            "",
            "record Circle(long radius) implements Shape {}",
            "",
            "record Square(long side) implements Shape {",
            "    long area() {",
            "        return side * side;",
            "    } // {",
            "}",
            ""));
        check(failed, work, "compact", String.join("\n",
            "package union_compact_comment;",
            "",
            "public final class Shapes {",
            "    public static long one() {",
            "        return 1L;",
            "    }",
            "}",
            "",
            "sealed interface Shape permits Circle, Square {}",
            "",
            "record Circle(long radius) implements Shape {}",
            "",
            "record Square(long side) implements Shape {",
            "    Square {",
            "        if (side < 0) {",
            "            throw new IllegalArgumentException(\"side\");",
            "        }",
            "    } // a side is never negative {",
            "}",
            ""));
        String[][] others = {
            {"accessor", "    public long side() {", "        return side;", "    } // {"},
            {"canonical", "    Square(long side) {", "        this.side = side;", "    } // {"},
            {"static-field", "    static final long UNIT = 1L; // {"},
            {"static-method", "    static Square unit() {", "        return new Square(1L);", "    } // {"},
            {"nested", "    record Twice(long side) {} // {"},
            {"initializer", "    static {", "    } // {"},
        };
        for (String[] o : others) {
            List<String> lines = new ArrayList<>(List.of(
                "package union_" + o[0].replace('-', '_') + "_comment;",
                "",
                "public final class Shapes {",
                "    public static long one() {",
                "        return 1L;",
                "    }",
                "}",
                "",
                "sealed interface Shape permits Circle, Square {}",
                "",
                "record Circle(long radius) implements Shape {}",
                "",
                "record Square(long side) implements Shape {"));
            lines.addAll(List.of(o).subList(1, o.length));
            lines.add("}");
            lines.add("");
            check(failed, work, o[0], String.join("\n", lines));
        }
        int all = 2 + others.length;
        for (String f : failed) {
            System.out.println("FAIL " + f);
        }
        System.out.println((all - failed.size()) + " passed, " + failed.size() + " failed");
        System.exit(failed.isEmpty() ? 0 : 1);
    }

    /**
     * The record Square declares a member, so the sealed interface is no union: the read must hold
     * no union and a refusal at Square's line, as {@code testdata/refuse/union-member} requires for
     * the same record without the comment.
     */
    @SuppressWarnings("unchecked")
    private static void check(List<String> failed, Path work, String name, String source) throws Exception {
        Path dir = work.resolve(name);
        Files.createDirectories(dir);
        Files.writeString(dir.resolve("Shapes.java"), source, StandardCharsets.UTF_8);
        Map<String, Object> doc = Reader.read(dir);
        boolean union = false;
        for (Object t : (List<Object>) doc.getOrDefault("types", List.of())) {
            union |= "union".equals(((Map<String, Object>) t).get("k"));
        }
        String square = "Shapes.java:" + (source.substring(0, source.indexOf("record Square")).split("\n", -1).length);
        boolean atSquare = false;
        for (Object r : (List<Object>) doc.getOrDefault("refusals", List.of())) {
            atSquare |= square.equals(((Map<String, Object>) r).get("pos"));
        }
        String line = "a_record_with_a_member_hidden_by_a_trailing_brace_comment_is_refused_at_its_line[" + name + "]";
        if (union || !atSquare) {
            failed.add(line + ": union read " + union + ", refusals " + doc.get("refusals")
                + "; want one at " + square);
        } else {
            System.out.println("ok " + line);
        }
    }
}

package codespec.jvm;

import codespec.jvm.JvmTypes.Primitive;
import codespec.jvm.JvmTypes.Type;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Function;

/**
 * The JVM type table's tests, run as a plain program:
 *
 * <pre>java -cp &lt;classes&gt; codespec.jvm.JvmTypesTest</pre>
 *
 * It prints one line per case and a summary {@code N passed, M failed}, and exits 1 when any case
 * fails. Expected types are written as {@code LinkedHashMap.toString()} prints them, so the field
 * order the JSON writer keeps is checked too.
 */
public final class JvmTypesTest {
    private static final String I32 = "{k=int, width=32, signed=true}";
    private static final String I64 = "{k=int, width=64, signed=true}";
    private static final String I16 = "{k=int, width=16, signed=true}";
    private static final String I8 = "{k=int, width=8, signed=true}";
    private static final String BOOL = "{k=bool}";
    private static final String STRING = "{k=string}";

    private JvmTypesTest() {}

    interface Case {
        void run() throws Exception;
    }

    public static void main(String[] args) {
        Map<String, Case> cases = new LinkedHashMap<>();
        cases.put("primitives_and_their_boxes_are_signed_integers_and_bool", JvmTypesTest::primitivesAndBoxes);
        cases.put("string_lists_and_maps_map_as_the_java_reader_maps_them", JvmTypesTest::libraryTypes);
        cases.put("double_char_big_decimal_and_object_are_refused_without_the_callback", JvmTypesTest::refused);
        cases.put("a_refused_type_argument_is_named_in_the_reason", JvmTypesTest::refusedArgument);
        cases.put("a_class_of_the_readers_own_goes_to_the_callback_whatever_its_package", JvmTypesTest::ownClass);
        cases.put("the_integer_boxes_are_the_four_boxes", JvmTypesTest::integerBoxes);
        cases.put("a_reference_field_left_unassigned_is_a_pointer_unless_its_type_already_holds_nil", JvmTypesTest::unassignedField);
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

    private static void primitivesAndBoxes() {
        expect(prim(Primitive.INT, "int"), I32);
        expect(prim(Primitive.LONG, "long"), I64);
        expect(prim(Primitive.SHORT, "short"), I16);
        expect(prim(Primitive.BYTE, "byte"), I8);
        expect(prim(Primitive.BOOLEAN, "boolean"), BOOL);
        expect(cls("java.lang.Integer"), I32);
        expect(cls("java.lang.Long"), I64);
        expect(cls("java.lang.Short"), I16);
        expect(cls("java.lang.Byte"), I8);
        expect(cls("java.lang.Boolean"), BOOL);
    }

    private static void libraryTypes() {
        expect(cls("java.lang.String"), STRING);
        expect(cls("java.util.List", cls("java.lang.String")), "{k=slice, elem=" + STRING + "}");
        expect(cls("java.util.ArrayList", cls("java.lang.Long")), "{k=slice, elem=" + I64 + "}");
        expect(cls("java.util.Map", cls("java.lang.String"), cls("java.lang.Integer")),
            "{k=map, key=" + STRING + ", elem=" + I32 + "}");
        expect(cls("java.util.HashMap", cls("java.lang.String"), cls("java.lang.Long")),
            "{k=map, key=" + STRING + ", elem=" + I64 + "}");
        expect(cls("java.util.TreeMap", cls("java.lang.String"), cls("java.lang.Boolean")),
            "{k=map, key=" + STRING + ", elem=" + BOOL + "}");
        expect(cls("java.util.Map", cls("java.lang.String"), cls("java.util.List", cls("java.lang.Integer"))),
            "{k=map, key=" + STRING + ", elem={k=slice, elem=" + I32 + "}}");
    }

    private static void refused() {
        refuse(prim(Primitive.DOUBLE, "double"), "type double is outside the reader's types");
        refuse(prim(Primitive.CHAR, "char"), "type char is outside the reader's types");
        refuse(cls("java.math.BigDecimal"), "type java.math.BigDecimal is outside the reader's types");
        refuse(cls("java.lang.Object"), "type java.lang.Object is outside the reader's types");
        refuse(Type.other("int[]"), "type int[] is outside the reader's types");
        refuse(Type.declared("java.util.List", List.of(), "java.util.List"), "type java.util.List is outside the reader's types");
    }

    private static void refusedArgument() {
        refuse(cls("java.util.Map", cls("java.lang.String"), cls("java.util.List", cls("java.lang.Double"))),
            "type java.lang.Double is outside the reader's types");
    }

    private static void ownClass() {
        List<String> asked = new ArrayList<>();
        Function<Type, Map<String, Object>> own = t -> {
            asked.add(t.className());
            if (!t.className().endsWith(".Book")) {
                return null;
            }
            Map<String, Object> named = new LinkedHashMap<>();
            named.put("k", "named");
            named.put("name", "Book");
            return named;
        };
        String got = JvmTypes.map(cls("java.util.List", cls("library.Book")), own).toString();
        check(got.equals("{k=slice, elem={k=named, name=Book}}"), "got " + got);
        String why = null;
        try {
            JvmTypes.map(cls("library.Shelf"), own);
        } catch (JvmTypes.Outside o) {
            why = o.getMessage();
        }
        check("type library.Shelf is outside the reader's types".equals(why), "reason " + why);
        for (String pkg : List.of("javax.lending", "java.lending")) {
            got = JvmTypes.map(cls(pkg + ".Book"), own).toString();
            check(got.equals("{k=named, name=Book}"), pkg + ".Book: got " + got);
        }
        why = null;
        try {
            JvmTypes.map(cls("java.util.Set", cls("java.lang.String")), own);
        } catch (JvmTypes.Outside o) {
            why = o.getMessage();
        }
        check("type java.util.Set<java.lang.String> is outside the reader's types".equals(why), "reason " + why);
        check(asked.equals(List.of("library.Book", "library.Shelf", "javax.lending.Book", "java.lending.Book", "java.util.Set")),
            "asked " + asked);
    }

    private static void integerBoxes() {
        for (String q : List.of("java.lang.Integer", "java.lang.Long", "java.lang.Short", "java.lang.Byte")) {
            check(JvmTypes.isIntegerBox(q), q + " is not a box");
        }
        for (String q : List.of("java.lang.Boolean", "java.lang.String", "java.lang.Double", "int")) {
            check(!JvmTypes.isIntegerBox(q), q + " is a box");
        }
    }

    /**
     * A reference field some constructor leaves unassigned holds null: a String, a box, a list, a map
     * or an enum is a pointer to its mapped type; a class the reader maps to a pointer, and an error,
     * already hold nil and stay as they are.
     */
    private static void unassignedField() {
        Function<Type, Map<String, Object>> own = t -> {
            Map<String, Object> named = new LinkedHashMap<>();
            named.put("k", "named");
            named.put("name", t.className().substring(t.className().lastIndexOf('.') + 1));
            if (t.className().endsWith(".Shade")) {
                return named;
            }
            Map<String, Object> ptr = new LinkedHashMap<>();
            ptr.put("k", "ptr");
            ptr.put("elem", named);
            return ptr;
        };
        Map<String, String> want = new LinkedHashMap<>();
        want.put("java.lang.String", "{k=ptr, elem=" + STRING + "}");
        want.put("java.lang.Integer", "{k=ptr, elem=" + I32 + "}");
        want.put("java.lang.Boolean", "{k=ptr, elem=" + BOOL + "}");
        want.put("library.Shade", "{k=ptr, elem={k=named, name=Shade}}");
        want.put("library.Book", "{k=ptr, elem={k=named, name=Book}}");
        for (Map.Entry<String, String> w : want.entrySet()) {
            String got = JvmTypes.unassignedField(JvmTypes.map(cls(w.getKey()), own)).toString();
            check(got.equals(w.getValue()), w.getKey() + ": got " + got + ", want " + w.getValue());
        }
        String list = JvmTypes.unassignedField(JvmTypes.map(cls("java.util.List", cls("java.lang.Long")), own)).toString();
        check(list.equals("{k=ptr, elem={k=slice, elem=" + I64 + "}}"), "List<Long>: got " + list);
        Map<String, Object> error = new LinkedHashMap<>();
        error.put("k", "error");
        check(JvmTypes.unassignedField(error).toString().equals("{k=error}"), "error: got " + JvmTypes.unassignedField(error));
        Map<String, Object> string = JvmTypes.map(cls("java.lang.String"), JvmTypesTest::never);
        JvmTypes.unassignedField(string);
        check(string.toString().equals(STRING), "the mapped type was changed: " + string);
    }

    // ---- helpers ----

    private static Type prim(Primitive k, String text) {
        return Type.primitive(k, text);
    }

    /** {@code name<args>}, printed as javac prints a declared type. */
    private static Type cls(String name, Type... args) {
        StringBuilder text = new StringBuilder(name);
        if (args.length > 0) {
            text.append('<');
            for (int i = 0; i < args.length; i++) {
                text.append(i == 0 ? "" : ",").append(args[i].text());
            }
            text.append('>');
        }
        return Type.declared(name, List.of(args), text.toString());
    }

    /** A callback that fails the case when it is consulted. */
    private static Map<String, Object> never(Type t) {
        throw new AssertionError("callback consulted for " + t.text());
    }

    private static void expect(Type t, String want) {
        String got = JvmTypes.map(t, JvmTypesTest::never).toString();
        check(got.equals(want), t.text() + ": got " + got + ", want " + want);
    }

    private static void refuse(Type t, String reason) {
        try {
            Object got = JvmTypes.map(t, JvmTypesTest::never);
            throw new AssertionError(t.text() + " mapped to " + got);
        } catch (JvmTypes.Outside o) {
            check(o.getMessage().equals(reason), t.text() + ": reason " + o.getMessage() + ", want " + reason);
        }
    }

    private static void check(boolean ok, String what) {
        if (!ok) {
            throw new AssertionError(what);
        }
    }
}

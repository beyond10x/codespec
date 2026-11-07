package codespec.jvm;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.function.Function;

/**
 * The JVM type table every JVM reader shares: a JVM type, described without javac's or Groovy's
 * type model, to its cir/1 type.
 *
 * <ul>
 *   <li>{@code int}, {@code long}, {@code short}, {@code byte} and their boxes {@code Integer},
 *       {@code Long}, {@code Short} and {@code Byte} are signed integers of 32, 64, 16 and 8 bits;
 *       {@code boolean} and {@code Boolean} are {@code bool}; {@code String} is {@code string}.
 *   <li>{@code List<T>} and {@code ArrayList<T>} are slices of T; {@code Map<K, V>}, {@code
 *       HashMap<K, V>} and {@code TreeMap<K, V>} maps from K to V. The type arguments are mapped by
 *       the same table, first to last.
 *   <li>{@code Object}, {@code Double}, {@code Float}, {@code Character}, {@code BigDecimal} and a
 *       raw or wrongly parameterised list or map class are refused outright. Every other class goes
 *       to the reader's callback, whatever its package, which maps it (a class of the package, an
 *       enum, an interface with one implementation, an error hierarchy) or returns {@code null} to
 *       refuse it.
 * </ul>
 *
 * <p>Every other type is refused with the reason {@code type <text> is outside the reader's
 * types}, {@code <text>} being the reader's own printing of the type ({@code
 * TypeMirror.toString()} in the Java reader). A refusal inside a type argument names that argument.
 *
 * <p>A reference field some constructor leaves unassigned holds null, which no zero value of its
 * mapped type is; {@link #unassignedField} gives its type, a pointer to the mapped type.
 */
public final class JvmTypes {
    private JvmTypes() {}

    /** A primitive type's kind. */
    public enum Primitive {
        BOOLEAN,
        BYTE,
        SHORT,
        INT,
        LONG,
        CHAR,
        FLOAT,
        DOUBLE
    }

    /**
     * A JVM type as a reader describes it: a primitive, a class by qualified name with its type
     * arguments, or any other type (an array, a type variable, a wildcard). {@code text} is how the
     * reader prints the type, and is what a refusal names.
     */
    public static final class Type {
        private final Primitive primitive;
        private final String className;
        private final List<Type> args;
        private final String text;

        private Type(Primitive primitive, String className, List<Type> args, String text) {
            this.primitive = primitive;
            this.className = className;
            this.args = args;
            this.text = text;
        }

        /** A primitive type. */
        public static Type primitive(Primitive kind, String text) {
            return new Type(kind, null, List.of(), text);
        }

        /** A class or interface type: its qualified name and its type arguments, none for a raw type. */
        public static Type declared(String qualifiedName, List<Type> args, String text) {
            return new Type(null, qualifiedName, List.copyOf(args), text);
        }

        /** Any other type, which the table refuses. */
        public static Type other(String text) {
            return new Type(null, null, List.of(), text);
        }

        /** The qualified class name, or {@code null} when the type is not a class or interface type. */
        public String className() {
            return className;
        }

        public List<Type> args() {
            return args;
        }

        public String text() {
            return text;
        }
    }

    /** A type outside the table; its message is the refusal reason. */
    public static final class Outside extends RuntimeException {
        private static final long serialVersionUID = 1L;

        Outside(String reason) {
            super(reason, null, false, false);
        }
    }

    private static final Map<String, String> BOXES = Map.of(
        "java.lang.Integer", "32", "java.lang.Long", "64", "java.lang.Short", "16", "java.lang.Byte", "8");

    /**
     * Classes the table refuses without asking the reader: {@code Object}, the floating-point and
     * {@code char} boxes, {@code BigDecimal}, and the table's own classes when they are raw or carry
     * the wrong number of type arguments. Every other class goes to the reader's callback, whatever
     * its package: a package of the reader's own may be named {@code javax.lending}.
     */
    private static final Set<String> REFUSED = Set.of(
        "java.lang.Object", "java.lang.Double", "java.lang.Float", "java.lang.Character",
        "java.math.BigDecimal", "java.util.List", "java.util.ArrayList", "java.util.Map",
        "java.util.HashMap", "java.util.TreeMap");

    /** Whether {@code qualifiedName} is one of the integer boxes {@code Integer}, {@code Long}, {@code Short}, {@code Byte}. */
    public static boolean isIntegerBox(String qualifiedName) {
        return BOXES.containsKey(qualifiedName);
    }

    /** The refusal reason for a type the table does not map. */
    public static String outside(String text) {
        return "type " + text + " is outside the reader's types";
    }

    /**
     * The cir/1 type of {@code t}, as a JSON object in the model's field order.
     *
     * @param own maps a class the reader declares itself, or returns {@code null} when the reader
     *     does not know it; not consulted for the classes the table refuses outright
     * @throws Outside when the table refuses the type or one of its type arguments
     */
    public static Map<String, Object> map(Type t, Function<Type, Map<String, Object>> own) {
        if (t.primitive != null) {
            switch (t.primitive) {
                case INT:
                    return tInt("32");
                case LONG:
                    return tInt("64");
                case SHORT:
                    return tInt("16");
                case BYTE:
                    return tInt("8");
                case BOOLEAN:
                    return kind("bool");
                default:
                    throw new Outside(outside(t.text));
            }
        }
        String qn = t.className;
        if (qn == null) {
            throw new Outside(outside(t.text));
        }
        List<Type> args = t.args;
        if (qn.equals("java.lang.String")) {
            return kind("string");
        }
        if (BOXES.containsKey(qn)) {
            return tInt(BOXES.get(qn));
        }
        if (qn.equals("java.lang.Boolean")) {
            return kind("bool");
        }
        if ((qn.equals("java.util.List") || qn.equals("java.util.ArrayList")) && args.size() == 1) {
            Map<String, Object> s = kind("slice");
            s.put("elem", map(args.get(0), own));
            return s;
        }
        if ((qn.equals("java.util.Map") || qn.equals("java.util.HashMap") || qn.equals("java.util.TreeMap")) && args.size() == 2) {
            Map<String, Object> m = kind("map");
            m.put("key", map(args.get(0), own));
            m.put("elem", map(args.get(1), own));
            return m;
        }
        if (!REFUSED.contains(qn)) {
            Map<String, Object> r = own.apply(t);
            if (r != null) {
                return r;
            }
        }
        throw new Outside(outside(t.text));
    }

    /**
     * The cir/1 type of a reference field some constructor leaves unassigned, given {@code mapped},
     * the type {@link #map} gives its JVM type: Java holds null there until it is assigned, which is
     * {@code nil}, so a type without nil ({@code string}, an integer, {@code bool}, a slice, a map,
     * an enum) becomes {@code ptr(mapped)}. A pointer (a class of the reader's own) and {@code
     * error} already hold nil and are returned as they are. Which fields are left unassigned is each
     * reader's analysis. {@code mapped} is not changed.
     */
    public static Map<String, Object> unassignedField(Map<String, Object> mapped) {
        Object k = mapped.get("k");
        if ("ptr".equals(k) || "error".equals(k)) {
            return mapped;
        }
        Map<String, Object> p = kind("ptr");
        p.put("elem", mapped);
        return p;
    }

    private static Map<String, Object> kind(String k) {
        Map<String, Object> t = new LinkedHashMap<>();
        t.put("k", k);
        return t;
    }

    private static Map<String, Object> tInt(String width) {
        Map<String, Object> t = kind("int");
        t.put("width", width);
        t.put("signed", true);
        return t;
    }
}

package codespec.read;

import com.sun.source.tree.AnnotationTree;
import com.sun.source.tree.AssignmentTree;
import com.sun.source.tree.BinaryTree;
import com.sun.source.tree.BlockTree;
import com.sun.source.tree.BreakTree;
import com.sun.source.tree.CaseTree;
import com.sun.source.tree.ClassTree;
import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.CompoundAssignmentTree;
import com.sun.source.tree.ContinueTree;
import com.sun.source.tree.EnhancedForLoopTree;
import com.sun.source.tree.ExpressionStatementTree;
import com.sun.source.tree.ExpressionTree;
import com.sun.source.tree.ForLoopTree;
import com.sun.source.tree.IdentifierTree;
import com.sun.source.tree.IfTree;
import com.sun.source.tree.LiteralTree;
import com.sun.source.tree.MemberSelectTree;
import com.sun.source.tree.MethodInvocationTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.tree.ModifiersTree;
import com.sun.source.tree.NewClassTree;
import com.sun.source.tree.ParenthesizedTree;
import com.sun.source.tree.ReturnTree;
import com.sun.source.tree.StatementTree;
import com.sun.source.tree.SwitchTree;
import com.sun.source.tree.ThrowTree;
import com.sun.source.tree.Tree;
import com.sun.source.tree.UnaryTree;
import com.sun.source.tree.VariableTree;
import com.sun.source.tree.WhileLoopTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.SourcePositions;
import com.sun.source.util.TreePath;
import com.sun.source.util.TreeScanner;
import com.sun.source.util.TreePathScanner;
import codespec.jvm.JvmTypes;
import com.sun.source.util.Trees;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.Deque;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;
import javax.lang.model.element.Element;
import javax.lang.model.element.ElementKind;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.Modifier;
import javax.lang.model.element.TypeElement;
import javax.lang.model.element.VariableElement;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.JavaCompiler;
import javax.tools.JavaFileObject;
import javax.tools.StandardJavaFileManager;
import javax.tools.StandardLocation;
import javax.tools.ToolProvider;

/**
 * The Java frontend: one package directory to one cir/1 module, from javac's attributed trees.
 *
 * <p>What it maps:
 *
 * <ul>
 *   <li>A class extending {@code Object} becomes a struct of its instance fields, its instance
 *       methods methods with the receiver {@code this} of type {@code *Class}, its static methods
 *       functions, its constructor the function {@code New<Class>} returning {@code *Class}, and its
 *       {@code static final} compile-time constants consts. A value of a package class type is a
 *       reference, {@code ptr(named)}.
 *   <li>An enum without members of its own becomes an enum over {@code string} whose variants hold
 *       their own names, as {@code name()} and {@code toString()} give them.
 *   <li>One checked-exception hierarchy: its root (a class extending {@code Exception} whose
 *       constructors only pass a message on) is {@code error}; each leaf (one constructor without
 *       parameters calling {@code super("<literal>")}) is a sentinel error. A method declaring it
 *       returns a trailing {@code error}; {@code throw new Leaf()} returns zero values and the
 *       sentinel; a call of such a method as a statement or a local's initializer checks the error
 *       and returns it.
 *   <li>{@code Optional<T>} as a method result is the comma-ok pair {@code (T, bool)}.
 *   <li>An interface with exactly one implementation in the package stands for that class.
 *   <li>By the JVM type table every JVM reader shares, {@link JvmTypes} under {@code adapters/jvm}:
 *       {@code int}, {@code long}, {@code short}, {@code byte} and their boxes are signed integers
 *       of 32, 64, 16 and 8 bits; {@code List}/{@code ArrayList} slices; {@code Map}, {@code
 *       HashMap} and {@code TreeMap} maps. A loop over a {@code HashMap}'s keys is a {@code
 *       for_range} over the map, whose order is unspecified, as Java's is; a loop over a {@code
 *       TreeMap}'s keys ranges over {@code sorted_keys(map)}, the keys in ascending order; a loop
 *       over a map whose class is not known from its static type is refused. {@code sorted_keys} is
 *       read only for String or integer keys of a local or field every assignment of which is
 *       {@code new TreeMap<>()}, so the order is the keys' natural one.
 *   <li>{@code add}, {@code set} and {@code put} are read only on a collection no other name
 *       shares: a field of {@code this}, or a local built with {@code new ...<>()} that is otherwise
 *       only a receiver, iterated or returned. A cir/1 slice or map write is not seen through
 *       another name, as Java's is.
 *   <li>{@code String.format} is read only with a literal format of {@code %s} of Strings, {@code
 *       %d} of integers and {@code %%}, which Formatter and {@code sprintf} print alike.
 *   <li>A field of a reference type that has no initializer and that some constructor does not
 *       assign first holds null until assigned, which no cir/1 zero value of its mapped type is: it
 *       is a pointer to that type ({@link JvmTypes#unassignedField}), whose zero value is {@code
 *       nil}; a class of the package is a pointer already. Such a field compared with {@code null}
 *       is its pointer compared with {@code nil}; assigned {@code null}, {@code nil}; assigned
 *       another such field, that field's pointer; assigned any other value, a pointer to a fresh
 *       local holding it. Every other read dereferences the pointer. A read Java would print
 *       (concatenation), unbox or compare (an operator, {@code +=}, {@code ++}) or hand to a library
 *       method as an argument, and a collection write to such a field, are refused at the use,
 *       since the written program stops at the nil pointer where Java goes on with null. Any other
 *       read assumes an object is there (README.md, Assumptions).
 *   <li>{@code ==} between two objects other than enum values is refused (identity); against
 *       {@code null} it is a nil check. A static member reached through an expression is refused,
 *       since Java evaluates and drops that expression.
 * </ul>
 *
 * <p>Everything else is kept as a {@code foreign} statement holding its Java source, or, for a
 * declaration cir/1 has no foreign form for, refused at its line; either way it is listed as a
 * refusal. Implicit numeric conversions are refused except for an integer literal, whose constant
 * value is typed as the context wants it.
 */
public final class Reader {
    /** Why a package could not be read at all. */
    public static final class Failure extends Exception {
        private static final long serialVersionUID = 1L;

        Failure(String message) {
            super(message);
        }
    }

    /** A construct outside cir/1; the enclosing statement becomes foreign with this reason. */
    private static final class Unsupported extends RuntimeException {
        private static final long serialVersionUID = 1L;

        Unsupported(String reason) {
            super(reason, null, false, false);
        }
    }

    private static final Set<String> HARMLESS_ANNOTATIONS = Set.of(
        "java.lang.Override", "java.lang.Deprecated", "java.lang.SuppressWarnings",
        "java.lang.FunctionalInterface", "java.lang.SafeVarargs");

    private final Trees trees;
    private final Types types;
    private final Elements elements;
    private final SourcePositions positions;
    private final Map<CompilationUnitTree, String> sources = new HashMap<>();
    /** Every compilation unit, in file order. */
    private final List<CompilationUnitTree> units = new ArrayList<>();

    /** Top-level type declarations, in file then source order. */
    private final List<TreePath> decls = new ArrayList<>();
    private final Set<TypeElement> structs = new HashSet<>();
    private final Set<TypeElement> enums = new HashSet<>();
    private final Set<TypeElement> interfaces = new HashSet<>();
    private final Map<TypeElement, TypeElement> implOf = new HashMap<>();
    private TypeElement errorRoot;
    private final Map<TypeElement, String> errorLeaves = new HashMap<>();
    private final Map<VariableElement, String> consts = new HashMap<>();
    /** The instance fields that hold null until assigned ({@link #nullableFields}), of every struct. */
    private final Set<VariableElement> unassigned = new HashSet<>();
    private final Map<ExecutableElement, Fn> fns = new HashMap<>();
    /** The constructor function of each struct, explicit or synthesized. */
    private final Map<TypeElement, Fn> ctors = new HashMap<>();

    private final List<Object> outTypes = new ArrayList<>();
    private final List<Object> outConsts = new ArrayList<>();
    private final List<Object> outErrors = new ArrayList<>();
    private final List<Object> outFuncs = new ArrayList<>();
    private final List<Object> layout = new ArrayList<>();
    private final List<Map<String, Object>> refusals = new ArrayList<>();

    /** A function or method the reader reads: its cir/1 signature and where it came from. */
    private static final class Fn {
        String name;
        boolean exported;
        boolean isStatic;
        boolean isCtor;
        TypeElement cls;
        List<String> paramNames = new ArrayList<>();
        List<Map<String, Object>> params = new ArrayList<>();
        /** Value results, without the trailing error and, for an Optional, without the bool. */
        List<Map<String, Object>> values = new ArrayList<>();
        boolean throwsErr;
        boolean optional;
        /** Declares a checked exception outside the package's hierarchy. */
        String foreignThrows;
        TreePath path;

        List<Map<String, Object>> results() {
            List<Map<String, Object>> r = new ArrayList<>(values);
            if (optional) {
                r.add(tBool());
            }
            if (throwsErr) {
                r.add(tError());
            }
            return r;
        }

        String key() {
            return isStatic || isCtor ? name : cls.getSimpleName() + "." + name;
        }
    }

    /** The state of one function body being read. */
    private final class Ctx {
        final Fn fn;
        final CompilationUnitTree cu;
        final boolean hasThis;
        final Set<String> used = new HashSet<>();
        final Deque<String> breaks = new ArrayDeque<>();
        /** The collections the enclosing for-each loops iterate, by the local or field naming them. */
        final Deque<Element> iterating = new ArrayDeque<>();

        Ctx(Fn fn, CompilationUnitTree cu, boolean hasThis) {
            this.fn = fn;
            this.cu = cu;
            this.hasThis = hasThis;
        }

        String fresh(String base) {
            String n = base;
            for (int i = 2; used.contains(n); i++) {
                n = base + i;
            }
            used.add(n);
            return n;
        }
    }

    private Reader(JavacTask task) {
        this.trees = Trees.instance(task);
        this.types = task.getTypes();
        this.elements = task.getElements();
        this.positions = trees.getSourcePositions();
    }

    /** Read every {@code .java} file directly in {@code dir}, all of one package. */
    public static Map<String, Object> read(Path dir) throws Failure {
        List<Path> files;
        try (Stream<Path> s = Files.list(dir)) {
            files = s.filter(p -> p.getFileName().toString().endsWith(".java") && Files.isRegularFile(p))
                .sorted(Comparator.comparing(p -> p.getFileName().toString()))
                .toList();
        } catch (IOException e) {
            throw new Failure(dir + ": " + e.getMessage());
        }
        if (files.isEmpty()) {
            throw new Failure(dir + ": no .java files");
        }
        JavaCompiler javac = ToolProvider.getSystemJavaCompiler();
        if (javac == null) {
            throw new Failure("this Java runtime has no compiler (module jdk.compiler): run on a JDK");
        }
        DiagnosticCollector<JavaFileObject> diags = new DiagnosticCollector<>();
        StandardJavaFileManager fm = javac.getStandardFileManager(diags, Locale.ROOT, StandardCharsets.UTF_8);
        List<CompilationUnitTree> units = new ArrayList<>();
        JavacTask task;
        try {
            fm.setLocation(StandardLocation.SOURCE_PATH, List.of());
            fm.setLocation(StandardLocation.CLASS_PATH, List.of());
            task = (JavacTask) javac.getTask(null, fm, diags,
                List.of("-proc:none", "-implicit:none", "-encoding", "UTF-8", "-Xlint:none", "-nowarn"),
                null, fm.getJavaFileObjectsFromPaths(files));
            for (CompilationUnitTree cu : task.parse()) {
                units.add(cu);
            }
            task.analyze();
        } catch (IOException e) {
            throw new Failure(dir + ": " + e.getMessage());
        }
        List<String> errors = new ArrayList<>();
        for (Diagnostic<? extends JavaFileObject> d : diags.getDiagnostics()) {
            if (d.getKind() == Diagnostic.Kind.ERROR) {
                String where = d.getSource() == null ? "" : Path.of(d.getSource().getName()).getFileName() + ":" + d.getLineNumber() + ": ";
                errors.add(where + d.getMessage(Locale.ROOT));
            }
        }
        if (!errors.isEmpty()) {
            errors.sort(null);
            throw new Failure(String.join("\n", errors));
        }
        units.sort(Comparator.comparing(cu -> fileName(cu)));
        Reader r = new Reader(task);
        return r.module(units);
    }

    private static String fileName(CompilationUnitTree cu) {
        return Path.of(cu.getSourceFile().getName()).getFileName().toString();
    }

    // ---- the module ----

    private Map<String, Object> module(List<CompilationUnitTree> units) throws Failure {
        String pkg = null;
        for (CompilationUnitTree cu : units) {
            String p = cu.getPackageName() == null ? "" : cu.getPackageName().toString();
            if (p.isEmpty()) {
                throw new Failure(fileName(cu) + ": no package declaration");
            }
            if (pkg != null && !pkg.equals(p)) {
                throw new Failure(fileName(cu) + ": package " + p + ", other files are in " + pkg);
            }
            pkg = p;
            this.units.add(cu);
            try {
                sources.put(cu, cu.getSourceFile().getCharContent(true).toString());
            } catch (IOException e) {
                throw new Failure(fileName(cu) + ": " + e.getMessage());
            }
            for (Tree t : cu.getTypeDecls()) {
                if (t instanceof ClassTree) {
                    decls.add(new TreePath(new TreePath(cu), t));
                }
            }
        }
        classify();
        signatures();
        for (TreePath p : decls) {
            emit(p);
        }
        refusals.sort(Comparator.comparing((Map<String, Object> r) -> file((String) r.get("pos")))
            .thenComparingInt(r -> line((String) r.get("pos"))));

        Map<String, Object> doc = Json.obj();
        doc.put("format", "cir/1");
        doc.put("name", pkg.substring(pkg.lastIndexOf('.') + 1));
        Map<String, Object> reader = Json.obj();
        reader.put("language", "java");
        reader.put("name", "codespec-read-java");
        reader.put("version", Main.VERSION);
        doc.put("reader", reader);
        doc.put("types", outTypes);
        if (!outConsts.isEmpty()) {
            doc.put("consts", outConsts);
        }
        if (!outErrors.isEmpty()) {
            doc.put("errors", outErrors);
        }
        doc.put("funcs", outFuncs);
        doc.put("layout", layout);
        if (!refusals.isEmpty()) {
            doc.put("refusals", new ArrayList<Object>(refusals));
        }
        return doc;
    }

    private static String file(String pos) {
        return pos.substring(0, pos.lastIndexOf(':'));
    }

    private static int line(String pos) {
        return Integer.parseInt(pos.substring(pos.lastIndexOf(':') + 1));
    }

    // ---- classification of the top-level declarations ----

    private TypeElement element(TreePath p) {
        return (TypeElement) trees.getElement(p);
    }

    private ClassTree classTree(TreePath p) {
        return (ClassTree) p.getLeaf();
    }

    private boolean inPackage(TypeElement te) {
        for (TreePath p : decls) {
            if (element(p).equals(te)) {
                return true;
            }
        }
        return false;
    }

    private String qname(TypeMirror t) {
        if (t.getKind() != TypeKind.DECLARED) {
            return "";
        }
        return ((TypeElement) ((DeclaredType) t).asElement()).getQualifiedName().toString();
    }

    private void classify() {
        Set<TypeElement> refused = new HashSet<>();
        // Shapes that do not depend on other declarations.
        for (TreePath p : decls) {
            TypeElement te = element(p);
            ClassTree ct = classTree(p);
            CompilationUnitTree cu = p.getCompilationUnit();
            refuseAnnotations(cu, ct.getModifiers());
            String why = null;
            switch (te.getKind()) {
                case ANNOTATION_TYPE:
                    why = "annotation type: annotations with behaviour are outside cir/1";
                    break;
                case INTERFACE:
                    why = interfaceShape(ct);
                    if (why == null) {
                        interfaces.add(te);
                    }
                    break;
                case ENUM:
                    why = enumShape(p);
                    if (why == null) {
                        enums.add(te);
                    }
                    break;
                case CLASS:
                    if (!te.getTypeParameters().isEmpty()) {
                        why = "generic class: generics declared by the code are outside cir/1";
                    } else if (te.getModifiers().contains(Modifier.ABSTRACT)) {
                        why = "abstract class: inheritance is outside cir/1";
                    }
                    break;
                default:
                    why = te.getKind() + " is outside cir/1";
            }
            if (why != null) {
                refuseDecl(cu, ct, ct.getModifiers(), te.getSimpleName().toString(), why);
                refused.add(te);
            }
        }
        // Error roots: classes extending java.lang.Exception.
        for (TreePath p : decls) {
            TypeElement te = element(p);
            if (refused.contains(te) || te.getKind() != ElementKind.CLASS) {
                continue;
            }
            if (!qname(te.getSuperclass()).equals("java.lang.Exception")) {
                continue;
            }
            String why = errorRoot != null ? "a second checked-exception hierarchy: one is read, " + errorRoot.getSimpleName()
                : errorRootShape(p);
            if (why != null) {
                refuseDecl(p.getCompilationUnit(), classTree(p), classTree(p).getModifiers(), te.getSimpleName().toString(), why);
                refused.add(te);
            } else {
                errorRoot = te;
            }
        }
        // Leaves of the hierarchy and plain classes.
        for (TreePath p : decls) {
            TypeElement te = element(p);
            if (refused.contains(te) || te.getKind() != ElementKind.CLASS || te.equals(errorRoot)) {
                continue;
            }
            String sup = qname(te.getSuperclass());
            String why = null;
            if (sup.equals("java.lang.Exception")) {
                continue; // a refused second root
            } else if (errorRoot != null && sup.equals(errorRoot.getQualifiedName().toString())) {
                String msg = errorLeafMessage(p);
                if (msg == null) {
                    why = "exception class other than one constructor without parameters calling super(\"<message>\")";
                } else {
                    errorLeaves.put(te, msg);
                    continue;
                }
            } else if (!sup.equals("java.lang.Object")) {
                why = "class extends " + sup + ": inheritance beyond one checked-exception hierarchy is outside cir/1";
            } else {
                for (TypeMirror i : te.getInterfaces()) {
                    TypeElement ie = (TypeElement) types.asElement(i);
                    if (!interfaces.contains(ie)) {
                        why = "implements " + qname(i) + ": only an interface of the package with one implementation is read";
                        break;
                    }
                    if (implOf.containsKey(ie)) {
                        why = "a second implementation of interface " + ie.getSimpleName() + " (the first is "
                            + implOf.get(ie).getSimpleName() + "): interfaces with more than one implementation are outside cir/1";
                        break;
                    }
                }
                if (why == null) {
                    for (TypeMirror i : te.getInterfaces()) {
                        implOf.put((TypeElement) types.asElement(i), te);
                    }
                    structs.add(te);
                    continue;
                }
            }
            refuseDecl(p.getCompilationUnit(), classTree(p), classTree(p).getModifiers(), te.getSimpleName().toString(), why);
            refused.add(te);
        }
        // A struct whose field types cannot be read is refused; that can make others unreadable.
        boolean changed = true;
        while (changed) {
            changed = false;
            for (TreePath p : decls) {
                TypeElement te = element(p);
                if (!structs.contains(te)) {
                    continue;
                }
                String why = structShape(p);
                if (why != null) {
                    structs.remove(te);
                    for (TypeElement i : new ArrayList<>(implOf.keySet())) {
                        if (implOf.get(i).equals(te)) {
                            implOf.remove(i);
                        }
                    }
                    refuseDecl(p.getCompilationUnit(), classTree(p), classTree(p).getModifiers(), te.getSimpleName().toString(), why);
                    changed = true;
                }
            }
        }
        for (TreePath p : decls) {
            if (structs.contains(element(p))) {
                for (VariableTree f : nullableFields(p)) {
                    unassigned.add((VariableElement) trees.getElement(new TreePath(p, f)));
                }
            }
        }
    }

    private String interfaceShape(ClassTree ct) {
        if (!ct.getTypeParameters().isEmpty()) {
            return "generic interface: generics declared by the code are outside cir/1";
        }
        if (!ct.getImplementsClause().isEmpty()) {
            return "interface extending another: inheritance is outside cir/1";
        }
        for (Tree m : ct.getMembers()) {
            if (!(m instanceof MethodTree) || ((MethodTree) m).getBody() != null
                || !((MethodTree) m).getTypeParameters().isEmpty()) {
                return "interface with a member other than an abstract method";
            }
        }
        return null;
    }

    private String enumShape(TreePath p) {
        ClassTree ct = classTree(p);
        if (!ct.getImplementsClause().isEmpty()) {
            return "enum implementing an interface";
        }
        for (Tree m : ct.getMembers()) {
            Element e = trees.getElement(new TreePath(p, m));
            if (e != null && e.getKind() == ElementKind.ENUM_CONSTANT) {
                NewClassTree init = (NewClassTree) ((VariableTree) m).getInitializer();
                if (init != null && (!init.getArguments().isEmpty() || init.getClassBody() != null)) {
                    return "enum constant with arguments or a body";
                }
                continue;
            }
            if (generated(e)) {
                continue;
            }
            return "enum with fields, constructors or methods of its own";
        }
        return null;
    }

    private boolean generated(Element e) {
        return e != null && elements.getOrigin(e) == Elements.Origin.MANDATED;
    }

    private boolean isSerialVersionUid(TreePath p, Tree m) {
        if (!(m instanceof VariableTree)) {
            return false;
        }
        Element e = trees.getElement(new TreePath(p, m));
        return e.getSimpleName().contentEquals("serialVersionUID") && e.getModifiers().contains(Modifier.STATIC);
    }

    /** The statements of a constructor body without javac's implicit {@code super()}. */
    private List<? extends StatementTree> ctorStatements(MethodTree mt) {
        return mt.getBody().getStatements();
    }

    private String errorRootShape(TreePath p) {
        ClassTree ct = classTree(p);
        if (!ct.getImplementsClause().isEmpty() || !ct.getTypeParameters().isEmpty()) {
            return "exception class with interfaces or type parameters";
        }
        for (Tree m : ct.getMembers()) {
            if (isSerialVersionUid(p, m)) {
                continue;
            }
            if (!(m instanceof MethodTree)) {
                return "exception class with a member other than constructors";
            }
            MethodTree mt = (MethodTree) m;
            Element e = trees.getElement(new TreePath(p, m));
            if (e.getKind() != ElementKind.CONSTRUCTOR) {
                return "exception class with a method";
            }
            if (generated(e)) {
                continue;
            }
            List<? extends StatementTree> body = ctorStatements(mt);
            if (body.size() != 1 || superArgument(body.get(0)) == null) {
                return "exception constructor other than one passing its message to super";
            }
            ExpressionTree arg = superArgument(body.get(0));
            if (!(arg instanceof IdentifierTree) || mt.getParameters().size() != 1
                || !((IdentifierTree) arg).getName().equals(mt.getParameters().get(0).getName())) {
                return "exception constructor other than one passing its message parameter to super";
            }
        }
        return null;
    }

    /** The one argument of a {@code super(x)} statement, or null. */
    private static ExpressionTree superArgument(StatementTree s) {
        if (!(s instanceof ExpressionStatementTree)) {
            return null;
        }
        ExpressionTree e = ((ExpressionStatementTree) s).getExpression();
        if (!(e instanceof MethodInvocationTree)) {
            return null;
        }
        MethodInvocationTree mi = (MethodInvocationTree) e;
        if (!(mi.getMethodSelect() instanceof IdentifierTree)
            || !((IdentifierTree) mi.getMethodSelect()).getName().contentEquals("super") || mi.getArguments().size() != 1) {
            return null;
        }
        return mi.getArguments().get(0);
    }

    private static boolean isBareSuper(StatementTree s) {
        if (!(s instanceof ExpressionStatementTree)) {
            return false;
        }
        ExpressionTree e = ((ExpressionStatementTree) s).getExpression();
        return e instanceof MethodInvocationTree
            && ((MethodInvocationTree) e).getMethodSelect() instanceof IdentifierTree
            && ((IdentifierTree) ((MethodInvocationTree) e).getMethodSelect()).getName().contentEquals("super")
            && ((MethodInvocationTree) e).getArguments().isEmpty();
    }

    private String errorLeafMessage(TreePath p) {
        ClassTree ct = classTree(p);
        if (!ct.getImplementsClause().isEmpty() || !ct.getTypeParameters().isEmpty()) {
            return null;
        }
        String msg = null;
        for (Tree m : ct.getMembers()) {
            if (isSerialVersionUid(p, m)) {
                continue;
            }
            if (!(m instanceof MethodTree)) {
                return null;
            }
            MethodTree mt = (MethodTree) m;
            Element e = trees.getElement(new TreePath(p, m));
            if (e.getKind() != ElementKind.CONSTRUCTOR || generated(e) || msg != null || !mt.getParameters().isEmpty()) {
                return null;
            }
            List<? extends StatementTree> body = ctorStatements(mt);
            ExpressionTree arg = body.size() == 1 ? superArgument(body.get(0)) : null;
            if (!(arg instanceof LiteralTree) || !(((LiteralTree) arg).getValue() instanceof String)) {
                return null;
            }
            msg = (String) ((LiteralTree) arg).getValue();
        }
        return msg;
    }

    /**
     * Instance fields of a reference type (anything but a primitive) without an initializer that some
     * constructor does not assign before it does anything else: on such a path a fresh object holds
     * null in the field, and cir/1's zero value of its type ({@code ""}, {@code 0}, an empty slice) is
     * not null, so the field is read as a pointer ({@link #fieldType}). A class without a constructor
     * of its own assigns nothing.
     */
    private List<VariableTree> nullableFields(TreePath p) {
        List<TreePath> ctors = new ArrayList<>();
        for (Tree m : classTree(p).getMembers()) {
            if (m instanceof MethodTree) {
                Element e = trees.getElement(new TreePath(p, m));
                if (e.getKind() == ElementKind.CONSTRUCTOR && !generated(e)) {
                    ctors.add(new TreePath(p, m));
                }
            }
        }
        List<VariableTree> out = new ArrayList<>();
        for (Tree m : classTree(p).getMembers()) {
            if (!(m instanceof VariableTree) || ((VariableTree) m).getInitializer() != null) {
                continue;
            }
            Element e = trees.getElement(new TreePath(p, m));
            if (e.getModifiers().contains(Modifier.STATIC) || e.asType().getKind().isPrimitive()) {
                continue;
            }
            boolean assigned = !ctors.isEmpty();
            for (TreePath c : ctors) {
                assigned &= assignedFirst(c, e);
            }
            if (!assigned) {
                out.add((VariableTree) m);
            }
        }
        return out;
    }

    /**
     * Whether the constructor's leading run of {@code this.f = <value>} statements, each value reading
     * nothing of {@code this}, assigns {@code field}.
     */
    private boolean assignedFirst(TreePath ctor, Element field) {
        MethodTree mt = (MethodTree) ctor.getLeaf();
        TreePath body = new TreePath(ctor, mt.getBody());
        for (StatementTree s : mt.getBody().getStatements()) {
            if (isBareSuper(s)) {
                continue;
            }
            if (!(s instanceof ExpressionStatementTree)
                || !(((ExpressionStatementTree) s).getExpression() instanceof AssignmentTree)) {
                return false;
            }
            TreePath sp = new TreePath(body, s);
            AssignmentTree at = (AssignmentTree) ((ExpressionStatementTree) s).getExpression();
            TreePath ap = new TreePath(sp, at);
            TreePath vp = new TreePath(ap, at.getVariable());
            Element target = trees.getElement(vp);
            boolean ownField = target != null && target.getKind() == ElementKind.FIELD
                && !target.getModifiers().contains(Modifier.STATIC)
                && (at.getVariable() instanceof IdentifierTree || isThis(((MemberSelectTree) at.getVariable()).getExpression()));
            if (!ownField || readsThis(new TreePath(ap, at.getExpression()))) {
                return false;
            }
            if (target.equals(field)) {
                return true;
            }
        }
        return false;
    }

    private static boolean isThis(ExpressionTree e) {
        return e instanceof IdentifierTree && ((IdentifierTree) e).getName().contentEquals("this");
    }

    /** Whether an expression reads {@code this}: by name, through a field or through an instance method. */
    private boolean readsThis(TreePath p) {
        boolean[] found = {false};
        new TreePathScanner<Void, Void>() {
            @Override
            public Void visitIdentifier(IdentifierTree id, Void x) {
                Element e = trees.getElement(getCurrentPath());
                if (id.getName().contentEquals("this") || id.getName().contentEquals("super")
                    || e != null && (e.getKind() == ElementKind.FIELD || e.getKind() == ElementKind.METHOD)
                        && !e.getModifiers().contains(Modifier.STATIC)) {
                    found[0] = true;
                }
                return null;
            }

            @Override
            public Void visitLambdaExpression(com.sun.source.tree.LambdaExpressionTree l, Void x) {
                found[0] = true;
                return null;
            }
        }.scan(p, null);
        return found[0];
    }

    private String structShape(TreePath p) {
        for (Tree m : classTree(p).getMembers()) {
            if (m instanceof VariableTree) {
                Element e = trees.getElement(new TreePath(p, m));
                if (e.getModifiers().contains(Modifier.STATIC)) {
                    continue;
                }
                try {
                    typeOf(e.asType());
                } catch (Unsupported u) {
                    return "field " + e.getSimpleName() + ": " + u.getMessage();
                }
            } else if (m instanceof BlockTree) {
                return "initializer block: cir/1 has no constructor phase for it";
            }
        }
        return null;
    }

    // ---- signatures ----

    private void signatures() {
        Set<String> funcNames = new HashSet<>();
        for (TreePath p : decls) {
            TypeElement te = element(p);
            if (structs.contains(te)) {
                funcNames.add("New" + te.getSimpleName());
            }
        }
        for (TreePath p : decls) {
            TypeElement te = element(p);
            if (!structs.contains(te)) {
                continue;
            }
            CompilationUnitTree cu = p.getCompilationUnit();
            Set<String> methodNames = new HashSet<>();
            boolean explicitCtor = false;
            for (Tree m : classTree(p).getMembers()) {
                TreePath mp = new TreePath(p, m);
                if (m instanceof ClassTree) {
                    refuseDecl(cu, m, ((ClassTree) m).getModifiers(), ((ClassTree) m).getSimpleName().toString(),
                        "nested class: only top-level classes are read");
                    continue;
                }
                if (m instanceof VariableTree) {
                    VariableTree vt = (VariableTree) m;
                    refuseAnnotations(cu, vt.getModifiers());
                    VariableElement ve = (VariableElement) trees.getElement(mp);
                    if (!ve.getModifiers().contains(Modifier.STATIC)) {
                        continue;
                    }
                    String why = null;
                    if (!ve.getModifiers().contains(Modifier.FINAL) || ve.getConstantValue() == null) {
                        why = "static field other than a compile-time constant: module-level state is outside cir/1";
                    } else if (funcNames.contains(ve.getSimpleName().toString()) || consts.containsValue(ve.getSimpleName().toString())) {
                        why = "a second constant named " + ve.getSimpleName();
                    } else {
                        try {
                            typeOf(ve.asType());
                        } catch (Unsupported u) {
                            why = u.getMessage();
                        }
                    }
                    if (why != null) {
                        refuseDecl(cu, m, vt.getModifiers(), te.getSimpleName() + "." + vt.getName(), why);
                    } else {
                        consts.put(ve, ve.getSimpleName().toString());
                    }
                    continue;
                }
                if (!(m instanceof MethodTree)) {
                    continue;
                }
                MethodTree mt = (MethodTree) m;
                ExecutableElement ee = (ExecutableElement) trees.getElement(mp);
                if (generated(ee)) {
                    continue;
                }
                refuseAnnotations(cu, mt.getModifiers());
                for (VariableTree param : mt.getParameters()) {
                    refuseAnnotations(cu, param.getModifiers());
                }
                boolean ctor = ee.getKind() == ElementKind.CONSTRUCTOR;
                String name = ctor ? "New" + te.getSimpleName() : ee.getSimpleName().toString();
                String what = ctor ? te.getSimpleName() + "(…)" : te.getSimpleName() + "." + name;
                String why = null;
                if (ctor && explicitCtor) {
                    why = "a second constructor: overloading is outside cir/1";
                } else if (!ctor && methodNames.contains(name)) {
                    why = "a second method named " + name + ": overloading is outside cir/1";
                } else if (!ctor && ee.getModifiers().contains(Modifier.STATIC) && funcNames.contains(name)) {
                    why = "a second function named " + name + " in the package";
                } else if (!ee.getTypeParameters().isEmpty()) {
                    why = "generic method: generics declared by the code are outside cir/1";
                } else if (mt.getBody() == null) {
                    why = "method without a body";
                } else if (ee.isVarArgs()) {
                    why = "variable arity method";
                }
                Fn fn = null;
                if (why == null) {
                    try {
                        fn = signature(te, ee, mp, name);
                    } catch (Unsupported u) {
                        why = u.getMessage();
                    }
                }
                if (why != null) {
                    refuseDecl(cu, m, mt.getModifiers(), what, why);
                    continue;
                }
                fns.put(ee, fn);
                if (ctor) {
                    explicitCtor = true;
                    ctors.put(te, fn);
                } else {
                    methodNames.add(name);
                    if (fn.isStatic) {
                        funcNames.add(name);
                    }
                }
            }
            if (!explicitCtor) {
                Fn fn = new Fn();
                fn.name = "New" + te.getSimpleName();
                fn.exported = te.getModifiers().contains(Modifier.PUBLIC);
                fn.isCtor = true;
                fn.cls = te;
                fn.values.add(tPtr(tNamed(te.getSimpleName().toString())));
                fn.path = p;
                ctors.put(te, fn);
            }
        }
    }

    private Fn signature(TypeElement te, ExecutableElement ee, TreePath mp, String name) {
        Fn fn = new Fn();
        fn.name = name;
        fn.exported = ee.getModifiers().contains(Modifier.PUBLIC);
        fn.isStatic = ee.getModifiers().contains(Modifier.STATIC);
        fn.isCtor = ee.getKind() == ElementKind.CONSTRUCTOR;
        fn.cls = te;
        fn.path = mp;
        for (VariableElement v : ee.getParameters()) {
            fn.paramNames.add(v.getSimpleName().toString());
            fn.params.add(typeOf(v.asType()));
        }
        if (fn.isCtor) {
            fn.values.add(tPtr(tNamed(te.getSimpleName().toString())));
        } else if (ee.getReturnType().getKind() != TypeKind.VOID) {
            TypeMirror rt = ee.getReturnType();
            if (qname(rt).equals("java.util.Optional")) {
                List<? extends TypeMirror> args = ((DeclaredType) rt).getTypeArguments();
                if (args.size() != 1) {
                    throw new Unsupported("raw Optional");
                }
                fn.optional = true;
                fn.values.add(typeOf(args.get(0)));
            } else {
                fn.values.add(typeOf(rt));
            }
        }
        for (TypeMirror t : ee.getThrownTypes()) {
            if (isErrorType(t)) {
                fn.throwsErr = true;
            } else if (fn.foreignThrows == null) {
                fn.foreignThrows = qname(t);
            }
        }
        if (fn.optional && fn.throwsErr) {
            throw new Unsupported("an Optional result of a method that throws: (T, bool, error) is outside cir/1");
        }
        return fn;
    }

    private boolean isErrorType(TypeMirror t) {
        if (t.getKind() != TypeKind.DECLARED) {
            return false;
        }
        TypeElement te = (TypeElement) types.asElement(t);
        return te.equals(errorRoot) || errorLeaves.containsKey(te);
    }

    // ---- emitting declarations ----

    private void emit(TreePath p) {
        TypeElement te = element(p);
        ClassTree ct = classTree(p);
        CompilationUnitTree cu = p.getCompilationUnit();
        String name = te.getSimpleName().toString();
        if (enums.contains(te)) {
            Map<String, Object> d = Json.obj();
            d.put("k", "enum");
            d.put("name", name);
            d.put("exported", te.getModifiers().contains(Modifier.PUBLIC));
            d.put("under", tString());
            List<Object> variants = new ArrayList<>();
            for (Element e : te.getEnclosedElements()) {
                if (e.getKind() == ElementKind.ENUM_CONSTANT) {
                    Map<String, Object> v = Json.obj();
                    v.put("name", e.getSimpleName().toString());
                    v.put("value", lit(quote(e.getSimpleName().toString()), tString()));
                    variants.add(v);
                }
            }
            d.put("variants", variants);
            putDoc(d, te);
            d.put("pos", declPos(cu, ct, ct.getModifiers()));
            outTypes.add(d);
            layout.add("type:" + name);
            return;
        }
        if (errorLeaves.containsKey(te)) {
            Map<String, Object> d = Json.obj();
            d.put("name", name);
            d.put("message", errorLeaves.get(te));
            d.put("pos", declPos(cu, ct, ct.getModifiers()));
            outErrors.add(d);
            layout.add("error:" + name);
            return;
        }
        if (!structs.contains(te)) {
            return;
        }
        Map<String, Object> d = Json.obj();
        d.put("k", "struct");
        d.put("name", name);
        d.put("exported", te.getModifiers().contains(Modifier.PUBLIC));
        List<Object> fields = new ArrayList<>();
        for (Tree m : ct.getMembers()) {
            if (m instanceof VariableTree) {
                Element e = trees.getElement(new TreePath(p, m));
                if (!e.getModifiers().contains(Modifier.STATIC)) {
                    Map<String, Object> f = Json.obj();
                    f.put("name", e.getSimpleName().toString());
                    f.put("t", fieldType((VariableElement) e));
                    fields.add(f);
                }
            }
        }
        d.put("fields", fields);
        putDoc(d, te);
        d.put("pos", declPos(cu, ct, ct.getModifiers()));
        outTypes.add(d);
        layout.add("type:" + name);
        Fn ctor = ctors.get(te);
        if (ctor.path == p) {
            emitFunc(ctor);
        }
        for (Tree m : ct.getMembers()) {
            Element e = trees.getElement(new TreePath(p, m));
            if (m instanceof VariableTree && consts.containsKey(e)) {
                VariableElement ve = (VariableElement) e;
                Map<String, Object> c = Json.obj();
                Map<String, Object> t = typeOf(ve.asType());
                c.put("name", consts.get(ve));
                c.put("t", t);
                c.put("value", constLit(ve.getConstantValue(), t));
                c.put("pos", declPos(cu, m, ((VariableTree) m).getModifiers()));
                outConsts.add(c);
                layout.add("const:" + consts.get(ve));
            } else if (m instanceof MethodTree && fns.containsKey(e)) {
                emitFunc(fns.get(e));
            }
        }
    }

    private void putDoc(Map<String, Object> d, Element e) {
        String doc = elements.getDocComment(e);
        if (doc == null) {
            return;
        }
        List<String> lines = new ArrayList<>();
        for (String l : doc.split("\n", -1)) {
            lines.add(l.strip());
        }
        while (!lines.isEmpty() && lines.get(0).isEmpty()) {
            lines.remove(0);
        }
        while (!lines.isEmpty() && lines.get(lines.size() - 1).isEmpty()) {
            lines.remove(lines.size() - 1);
        }
        if (!lines.isEmpty()) {
            d.put("doc", String.join("\n", lines));
        }
    }

    private void emitFunc(Fn fn) {
        CompilationUnitTree cu = fn.path.getCompilationUnit();
        Ctx ctx = new Ctx(fn, cu, !fn.isStatic);
        ctx.used.add("this");
        ctx.used.addAll(fn.paramNames);
        Map<String, Object> f = Json.obj();
        f.put("name", fn.name);
        f.put("exported", fn.exported);
        if (!fn.isStatic && !fn.isCtor) {
            Map<String, Object> recv = Json.obj();
            recv.put("name", "this");
            recv.put("t", tPtr(tNamed(fn.cls.getSimpleName().toString())));
            f.put("recv", recv);
        }
        List<Object> params = new ArrayList<>();
        for (int i = 0; i < fn.params.size(); i++) {
            Map<String, Object> pm = Json.obj();
            pm.put("name", fn.paramNames.get(i));
            pm.put("t", fn.params.get(i));
            params.add(pm);
        }
        f.put("params", params);
        f.put("results", new ArrayList<Object>(fn.results()));
        List<Object> body = new ArrayList<>();
        String pos;
        if (fn.path.getLeaf() instanceof MethodTree) {
            MethodTree mt = (MethodTree) fn.path.getLeaf();
            pos = declPos(cu, mt, mt.getModifiers());
            new TreeScanner<Void, Void>() {
                @Override
                public Void visitVariable(VariableTree v, Void x) {
                    ctx.used.add(v.getName().toString());
                    return super.visitVariable(v, x);
                }
            }.scan(mt.getBody(), null);
            TreePath bodyPath = new TreePath(fn.path, mt.getBody());
            List<? extends StatementTree> stmts = mt.getBody().getStatements();
            if (fn.isCtor) {
                body.add(thisAlloc(fn, ctx, pos));
                if (!stmts.isEmpty() && isBareSuper(stmts.get(0))) {
                    stmts = stmts.subList(1, stmts.size());
                }
            }
            body.addAll(block(bodyPath, stmts, ctx));
            if ((fn.isCtor || (fn.values.isEmpty() && fn.throwsErr)) && (stmts.isEmpty() || !exits(stmts.get(stmts.size() - 1)))) {
                body.add(ret(successResults(ctx, null), pos));
            }
        } else {
            ClassTree ct = (ClassTree) fn.path.getLeaf();
            pos = declPos(cu, ct, ct.getModifiers());
            body.add(thisAlloc(fn, ctx, pos));
            body.add(ret(successResults(ctx, null), pos));
        }
        f.put("body", body);
        if (fn.path.getLeaf() instanceof MethodTree) {
            putDoc(f, trees.getElement(fn.path));
        }
        f.put("pos", pos);
        outFuncs.add(f);
        layout.add("func:" + fn.key());
    }

    /** {@code this := &Class{<field initializers>}}, the first statement of every constructor. */
    private Map<String, Object> thisAlloc(Fn fn, Ctx ctor, String pos) {
        TreePath classPath = fn.path.getLeaf() instanceof ClassTree ? fn.path : fn.path.getParentPath();
        CompilationUnitTree cu = classPath.getCompilationUnit();
        Ctx noThis = new Ctx(fn, cu, false);
        String name = fn.cls.getSimpleName().toString();
        List<Object> inits = new ArrayList<>();
        for (Tree m : classTree(classPath).getMembers()) {
            if (!(m instanceof VariableTree) || ((VariableTree) m).getInitializer() == null) {
                continue;
            }
            TreePath vp = new TreePath(classPath, m);
            Element e = trees.getElement(vp);
            if (e.getModifiers().contains(Modifier.STATIC)) {
                continue;
            }
            try {
                Map<String, Object> fi = Json.obj();
                fi.put("name", e.getSimpleName().toString());
                fi.put("value", coerce(expr(new TreePath(vp, ((VariableTree) m).getInitializer()), noThis), typeOf(e.asType())));
                checkTyped(fi);
                inits.add(fi);
            } catch (Unsupported u) {
                return foreign(cu, m, u.getMessage());
            }
        }
        Map<String, Object> lit = Json.obj();
        lit.put("k", "struct");
        lit.put("name", name);
        lit.put("fields", inits);
        lit.put("t", tNamed(name));
        Map<String, Object> s = Json.obj();
        s.put("k", "let");
        s.put("names", new ArrayList<Object>(List.of("this")));
        s.put("rhs", new ArrayList<Object>(List.of(unop("&", lit, tPtr(tNamed(name))))));
        s.put("pos", pos);
        return s;
    }

    /** Whether a statement never completes normally, judged on its shape. */
    private static boolean exits(StatementTree s) {
        if (s instanceof ReturnTree || s instanceof ThrowTree) {
            return true;
        }
        if (s instanceof BlockTree) {
            List<? extends StatementTree> l = ((BlockTree) s).getStatements();
            return !l.isEmpty() && exits(l.get(l.size() - 1));
        }
        if (s instanceof IfTree) {
            IfTree i = (IfTree) s;
            return i.getElseStatement() != null && exits(i.getThenStatement()) && exits(i.getElseStatement());
        }
        return false;
    }

    // ---- statements ----

    private List<Object> block(TreePath parent, List<? extends StatementTree> stmts, Ctx ctx) {
        List<Object> out = new ArrayList<>();
        for (StatementTree s : stmts) {
            out.addAll(stmt(new TreePath(parent, s), ctx));
        }
        return out;
    }

    /** The statements of a loop, if or case body: a block's statements, or the one statement. */
    private List<Object> body(TreePath parent, StatementTree s, Ctx ctx) {
        if (s instanceof BlockTree) {
            return block(new TreePath(parent, s), ((BlockTree) s).getStatements(), ctx);
        }
        return stmt(new TreePath(parent, s), ctx);
    }

    private List<Object> inside(String target, TreePath parent, StatementTree s, Ctx ctx) {
        ctx.breaks.push(target);
        try {
            return body(parent, s, ctx);
        } finally {
            ctx.breaks.pop();
        }
    }

    private List<Object> stmt(TreePath p, Ctx ctx) {
        try {
            List<Object> out = strict(p, ctx);
            for (Object o : out) {
                checkTyped(o);
            }
            return out;
        } catch (Unsupported u) {
            return List.of(foreign(ctx.cu, p.getLeaf(), u.getMessage()));
        }
    }

    private List<Object> strict(TreePath p, Ctx ctx) {
        StatementTree s = (StatementTree) p.getLeaf();
        String pos = pos(ctx.cu, s);
        switch (s.getKind()) {
            case VARIABLE:
                return local(p, ctx, pos);
            case EXPRESSION_STATEMENT:
                return exprStmt(new TreePath(p, ((ExpressionStatementTree) s).getExpression()), ctx, pos);
            case IF: {
                IfTree it = (IfTree) s;
                Map<String, Object> cond = coerce(expr(new TreePath(p, it.getCondition()), ctx), tBool());
                Map<String, Object> st = Json.obj();
                st.put("k", "if");
                st.put("cond", cond);
                st.put("then", body(p, it.getThenStatement(), ctx));
                if (it.getElseStatement() != null) {
                    st.put("else", body(p, it.getElseStatement(), ctx));
                }
                st.put("pos", pos);
                return List.of(st);
            }
            case WHILE_LOOP: {
                WhileLoopTree wt = (WhileLoopTree) s;
                Map<String, Object> cond = coerce(expr(new TreePath(p, wt.getCondition()), ctx), tBool());
                Map<String, Object> st = Json.obj();
                st.put("k", "for");
                if (!isTrue(cond)) {
                    st.put("cond", cond);
                }
                st.put("body", inside("loop", p, wt.getStatement(), ctx));
                st.put("pos", pos);
                return List.of(st);
            }
            case FOR_LOOP:
                return forLoop(p, ctx, pos);
            case ENHANCED_FOR_LOOP:
                return forEach(p, ctx, pos);
            case BREAK: {
                if (((BreakTree) s).getLabel() != null) {
                    throw new Unsupported("labeled break is outside cir/1");
                }
                if (!"loop".equals(ctx.breaks.peek())) {
                    throw new Unsupported("break out of a switch other than at the end of a case: cir/1 break leaves a loop");
                }
                return List.of(simple("break", pos));
            }
            case CONTINUE:
                if (((ContinueTree) s).getLabel() != null) {
                    throw new Unsupported("labeled continue is outside cir/1");
                }
                return List.of(simple("continue", pos));
            case SWITCH:
                return switchStmt(p, ctx, pos);
            case RETURN:
                return returnStmt(p, ctx, pos);
            case THROW:
                return throwStmt(p, ctx, pos);
            case SYNCHRONIZED:
                throw new Unsupported("synchronized: threads and locks are outside cir/1");
            case TRY:
                throw new Unsupported("try: catching exceptions is outside cir/1");
            case DO_WHILE_LOOP:
                throw new Unsupported("do-while is outside cir/1");
            case BLOCK:
                throw new Unsupported("a nested block is outside cir/1");
            case LABELED_STATEMENT:
                throw new Unsupported("labeled statement is outside cir/1");
            case CLASS:
                throw new Unsupported("local class is outside cir/1");
            default:
                throw new Unsupported(s.getKind().toString().toLowerCase(Locale.ROOT).replace('_', ' ') + " is outside cir/1");
        }
    }

    private static boolean isTrue(Map<String, Object> e) {
        return "lit".equals(e.get("k")) && "true".equals(e.get("value"));
    }

    private Map<String, Object> simple(String k, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", k);
        st.put("pos", pos);
        return st;
    }

    private List<Object> local(TreePath p, Ctx ctx, String pos) {
        VariableTree vt = (VariableTree) p.getLeaf();
        String name = vt.getName().toString();
        if (vt.getInitializer() != null) {
            TreePath ip = new TreePath(p, vt.getInitializer());
            Fn callee = throwingCallee(ip);
            if (callee != null) {
                Map<String, Object> t = typeOf(trees.getElement(p).asType());
                Map<String, Object> call = call(ip, ctx, true);
                if (callee.values.size() != 1 || !callee.values.get(0).equals(t)) {
                    throw new Unsupported("a call that throws, whose result is converted");
                }
                return propagate(call, name, callee, ctx, pos);
            }
            Map<String, Object> init = expr(ip, ctx);
            Map<String, Object> t = typeOf(trees.getElement(p).asType());
            if ("nil".equals(init.get("k"))) {
                return List.of(var(name, coerce(init, t).get("t"), pos));
            }
            Map<String, Object> st = Json.obj();
            st.put("k", "let");
            st.put("names", new ArrayList<Object>(List.of(name)));
            st.put("rhs", new ArrayList<Object>(List.of(coerce(init, t))));
            st.put("pos", pos);
            return List.of(st);
        }
        return List.of(var(name, typeOf(trees.getElement(p).asType()), pos));
    }

    private Map<String, Object> var(String name, Object t, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", "var");
        st.put("name", name);
        st.put("t", t);
        st.put("pos", pos);
        return st;
    }

    private List<Object> exprStmt(TreePath ep, Ctx ctx, String pos) {
        ExpressionTree e = (ExpressionTree) ep.getLeaf();
        switch (e.getKind()) {
            case METHOD_INVOCATION: {
                List<Object> lib = collectionWrite(ep, ctx, pos);
                if (lib != null) {
                    return lib;
                }
                Fn callee = throwingCallee(ep);
                if (callee != null) {
                    return propagate(call(ep, ctx, true), null, callee, ctx, pos);
                }
                Map<String, Object> st = Json.obj();
                st.put("k", "expr");
                st.put("x", expr(ep, ctx));
                st.put("pos", pos);
                return List.of(st);
            }
            case ASSIGNMENT: {
                AssignmentTree at = (AssignmentTree) e;
                VariableElement target = unassignedRead(new TreePath(ep, at.getVariable()));
                if (target != null) {
                    return assignUnassigned(target, new TreePath(ep, at.getVariable()), new TreePath(ep, at.getExpression()), ctx, pos);
                }
                Map<String, Object> lhs = place(new TreePath(ep, at.getVariable()), ctx);
                Map<String, Object> rhs = coerce(expr(new TreePath(ep, at.getExpression()), ctx), type(lhs));
                return List.of(assign("", lhs, rhs, pos));
            }
            case PLUS_ASSIGNMENT:
            case MINUS_ASSIGNMENT:
            case MULTIPLY_ASSIGNMENT:
            case DIVIDE_ASSIGNMENT:
            case REMAINDER_ASSIGNMENT:
            case AND_ASSIGNMENT:
            case OR_ASSIGNMENT:
            case XOR_ASSIGNMENT: {
                CompoundAssignmentTree ct = (CompoundAssignmentTree) e;
                refuseUnassigned(new TreePath(ep, ct.getVariable()), "the target of " + compoundOp(e.getKind()),
                    "Java unboxes or prints the null" + STOPS);
                refuseUnassigned(new TreePath(ep, ct.getExpression()), "an operand of " + compoundOp(e.getKind()),
                    "Java unboxes or prints the null" + STOPS);
                Map<String, Object> lhs = place(new TreePath(ep, ct.getVariable()), ctx);
                Map<String, Object> rhs = expr(new TreePath(ep, ct.getExpression()), ctx);
                String op = compoundOp(e.getKind());
                Map<String, Object> lt = type(lhs);
                if (lt.equals(tString())) {
                    if (!op.equals("+=") || !type(rhs).equals(tString())) {
                        throw new Unsupported("+= of a value other than a String to a String");
                    }
                } else if (!isInt(lt)) {
                    throw new Unsupported(op + " on a " + describe(lt));
                }
                return List.of(assign(op, lhs, coerce(rhs, lt), pos));
            }
            case POSTFIX_INCREMENT:
            case PREFIX_INCREMENT:
            case POSTFIX_DECREMENT:
            case PREFIX_DECREMENT: {
                UnaryTree ut = (UnaryTree) e;
                refuseUnassigned(new TreePath(ep, ut.getExpression()), "the target of ++ or --", "Java unboxes the null" + STOPS);
                Map<String, Object> x = place(new TreePath(ep, ut.getExpression()), ctx);
                if (!isInt(type(x))) {
                    throw new Unsupported("++ or -- on a " + describe(type(x)));
                }
                boolean inc = e.getKind() == Tree.Kind.POSTFIX_INCREMENT || e.getKind() == Tree.Kind.PREFIX_INCREMENT;
                Map<String, Object> st = Json.obj();
                st.put("k", inc ? "incr" : "decr");
                st.put("x", x);
                st.put("pos", pos);
                return List.of(st);
            }
            default:
                throw new Unsupported("expression statement " + e.getKind().toString().toLowerCase(Locale.ROOT).replace('_', ' ') + " is outside cir/1");
        }
    }

    private static String compoundOp(Tree.Kind k) {
        switch (k) {
            case PLUS_ASSIGNMENT:
                return "+=";
            case MINUS_ASSIGNMENT:
                return "-=";
            case MULTIPLY_ASSIGNMENT:
                return "*=";
            case DIVIDE_ASSIGNMENT:
                return "/=";
            case REMAINDER_ASSIGNMENT:
                return "%=";
            case AND_ASSIGNMENT:
                return "&=";
            case OR_ASSIGNMENT:
                return "|=";
            default:
                return "^=";
        }
    }

    private Map<String, Object> assign(String op, Map<String, Object> lhs, Map<String, Object> rhs, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", "assign");
        if (!op.isEmpty()) {
            st.put("op", op);
        }
        st.put("lhs", new ArrayList<Object>(List.of(lhs)));
        st.put("rhs", new ArrayList<Object>(List.of(rhs)));
        st.put("pos", pos);
        return st;
    }

    /**
     * {@code f = v} for a field that holds null until assigned, whose cir/1 field is a pointer: {@code
     * null} is {@code nil}; another such field of the same type gives its pointer, null or not; any
     * other value is bound to a fresh local and the field points at it, so no later write to the
     * value's own name changes the field. Nothing writes through such a pointer, so a pointer two
     * fields hold is never seen changed through the other.
     */
    private List<Object> assignUnassigned(VariableElement target, TreePath lp, TreePath rp, Ctx ctx, String pos) {
        Map<String, Object> lhs = unassignedPointer(lp, ctx);
        Map<String, Object> pt = fieldType(target);
        if (unparen(rp).getLeaf().getKind() == Tree.Kind.NULL_LITERAL) {
            return List.of(assign("", lhs, nil(pt), pos));
        }
        VariableElement source = unassignedRead(rp);
        if (source != null && fieldType(source).equals(pt)) {
            return List.of(assign("", lhs, unassignedPointer(rp, ctx), pos));
        }
        Map<String, Object> elem = typeOf(target.asType());
        Map<String, Object> value = coerce(expr(rp, ctx), elem);
        String name = ctx.fresh(target.getSimpleName().toString());
        Map<String, Object> let = Json.obj();
        let.put("k", "let");
        let.put("names", new ArrayList<Object>(List.of(name)));
        let.put("rhs", new ArrayList<Object>(List.of(value)));
        let.put("pos", pos);
        return List.of(let, assign("", lhs, unop("&", varRef(name, elem), pt), pos));
    }

    /** An expression that can be assigned to: a local, a field or a list or map element. */
    private Map<String, Object> place(TreePath p, Ctx ctx) {
        Map<String, Object> x = expr(p, ctx);
        String k = (String) x.get("k");
        if (!k.equals("var") && !k.equals("field")) {
            throw new Unsupported("assignment to something other than a local or a field");
        }
        return x;
    }

    /**
     * Refuse a String or boxed parameter printed as it is, by concatenation or {@code String.format},
     * in a method that never compares it with {@code null}: Java prints a null one as {@code null},
     * and a cir/1 string or integer has no null. Every other use of a parameter assumes it is not
     * null (README.md, Assumptions).
     */
    private void refusePrintedNullable(TreePath part, Ctx ctx) {
        part = unparen(part);
        Element e = trees.getElement(part);
        if (!(part.getLeaf() instanceof IdentifierTree) || e == null || e.getKind() != ElementKind.PARAMETER) {
            return;
        }
        String q = qname(e.asType());
        if (!q.equals("java.lang.String") && !q.equals("java.lang.Boolean") && !JvmTypes.isIntegerBox(q)) {
            return;
        }
        boolean[] checked = {false};
        new TreePathScanner<Void, Void>() {
            @Override
            public Void visitBinary(BinaryTree b, Void x) {
                if ((b.getKind() == Tree.Kind.EQUAL_TO || b.getKind() == Tree.Kind.NOT_EQUAL_TO)
                    && (b.getLeftOperand().getKind() == Tree.Kind.NULL_LITERAL || b.getRightOperand().getKind() == Tree.Kind.NULL_LITERAL)) {
                    ExpressionTree o = b.getLeftOperand().getKind() == Tree.Kind.NULL_LITERAL ? b.getRightOperand() : b.getLeftOperand();
                    if (e.equals(trees.getElement(new TreePath(getCurrentPath(), o)))) {
                        checked[0] = true;
                    }
                }
                return super.visitBinary(b, x);
            }
        }.scan(ctx.fn.path, null);
        if (!checked[0]) {
            throw new Unsupported("parameter " + e.getSimpleName() + " printed where Java prints a null one as null,"
                + " which a cir/1 " + (q.equals("java.lang.String") ? "string" : "integer") + " cannot hold; compare it"
                + " with null first");
        }
    }

    /** Whether a qualifier names a class, enum or interface rather than computing a value. */
    private boolean namesAType(TreePath p) {
        Element e = trees.getElement(p);
        return (p.getLeaf() instanceof IdentifierTree || p.getLeaf() instanceof MemberSelectTree)
            && e != null && (e.getKind().isClass() || e.getKind().isInterface());
    }

    private static TreePath unparen(TreePath p) {
        while (p.getLeaf() instanceof ParenthesizedTree) {
            p = new TreePath(p, ((ParenthesizedTree) p.getLeaf()).getExpression());
        }
        return p;
    }

    /**
     * Whether a collection is one only this name reaches: a field of {@code this}, or a local
     * initialised with {@code new ArrayList/HashMap/TreeMap<>()} whose every other use is as a
     * receiver ({@code l.m(…)}), the collection of a for-each or the value of a return.
     */
    private boolean owned(TreePath recv, Ctx ctx) {
        recv = unparen(recv);
        Tree t = recv.getLeaf();
        Element e = trees.getElement(recv);
        if (e == null) {
            return false;
        }
        if (e.getKind() == ElementKind.FIELD && !e.getModifiers().contains(Modifier.STATIC)) {
            return (t instanceof IdentifierTree || t instanceof MemberSelectTree && isThis(((MemberSelectTree) t).getExpression()))
                && ownedField(e);
        }
        if (e.getKind() != ElementKind.LOCAL_VARIABLE || !(ctx.fn.path.getLeaf() instanceof MethodTree)) {
            return false;
        }
        TreePath body = new TreePath(ctx.fn.path, ((MethodTree) ctx.fn.path.getLeaf()).getBody());
        boolean[] ok = {true};
        boolean[] built = {false};
        new TreePathScanner<Void, Void>() {
            @Override
            public Void visitVariable(VariableTree v, Void x) {
                if (e.equals(trees.getElement(getCurrentPath()))) {
                    built[0] = isNewCollection(v.getInitializer(), Set.of("java.util.ArrayList", "java.util.HashMap", "java.util.TreeMap"),
                        new TreePath(getCurrentPath(), v.getInitializer() == null ? v : v.getInitializer()));
                }
                return super.visitVariable(v, x);
            }

            @Override
            public Void visitIdentifier(IdentifierTree id, Void x) {
                if (e.equals(trees.getElement(getCurrentPath()))) {
                    Tree parent = getCurrentPath().getParentPath().getLeaf();
                    boolean receiver = parent instanceof MemberSelectTree && ((MemberSelectTree) parent).getExpression() == id;
                    boolean iterated = parent instanceof EnhancedForLoopTree && ((EnhancedForLoopTree) parent).getExpression() == id;
                    if (!receiver && !iterated && !(parent instanceof ReturnTree)) {
                        ok[0] = false;
                    }
                }
                return null;
            }
        }.scan(body, null);
        return ok[0] && built[0];
    }

    /**
     * Whether a map is a local or an instance field every assignment of which, its initializer
     * included, is {@code new TreeMap<>()}: a TreeMap with the keys' natural order. Locals are
     * searched in their method, fields in every file of the package.
     */
    private boolean naturalTreeMap(TreePath p) {
        p = unparen(p);
        Element e = trees.getElement(p);
        Tree t = p.getLeaf();
        if (e == null || !(t instanceof IdentifierTree || t instanceof MemberSelectTree)) {
            return false;
        }
        List<TreePath> scope = new ArrayList<>();
        if (e.getKind() == ElementKind.LOCAL_VARIABLE) {
            TreePath m = p;
            while (m != null && !(m.getLeaf() instanceof MethodTree)) {
                m = m.getParentPath();
            }
            if (m == null) {
                return false;
            }
            scope.add(m);
        } else if (e.getKind() == ElementKind.FIELD && !e.getModifiers().contains(Modifier.STATIC)) {
            for (CompilationUnitTree cu : units) {
                scope.add(new TreePath(cu));
            }
        } else {
            return false;
        }
        Set<String> tree = Set.of("java.util.TreeMap");
        int[] seen = {0};
        boolean[] ok = {true};
        for (TreePath s : scope) {
            new TreePathScanner<Void, Void>() {
                @Override
                public Void visitVariable(VariableTree v, Void x) {
                    if (e.equals(trees.getElement(getCurrentPath()))) {
                        if (v.getInitializer() != null) {
                            seen[0]++;
                            ok[0] &= isNewCollection(v.getInitializer(), tree, new TreePath(getCurrentPath(), v.getInitializer()));
                        }
                    }
                    return super.visitVariable(v, x);
                }

                @Override
                public Void visitAssignment(AssignmentTree a, Void x) {
                    if (e.equals(trees.getElement(new TreePath(getCurrentPath(), a.getVariable())))) {
                        seen[0]++;
                        ok[0] &= isNewCollection(a.getExpression(), tree, new TreePath(getCurrentPath(), a.getExpression()));
                    }
                    return super.visitAssignment(a, x);
                }
            }.scan(s, null);
        }
        return ok[0] && seen[0] > 0;
    }

    /**
     * Whether no name but the field reaches its collection, searched in every file of the package:
     * every assignment, the initializer included, is {@code new ArrayList/HashMap/TreeMap<>()}, and
     * every other use is as a receiver ({@code f.m(…)}) or as the collection of a for-each. A field
     * returned, passed as an argument or assigned to another name has escaped.
     */
    private boolean ownedField(Element field) {
        Set<String> fresh = Set.of("java.util.ArrayList", "java.util.HashMap", "java.util.TreeMap");
        int[] assigned = {0};
        boolean[] ok = {true};
        for (CompilationUnitTree cu : units) {
            new TreePathScanner<Void, Void>() {
                @Override
                public Void visitVariable(VariableTree v, Void x) {
                    if (field.equals(trees.getElement(getCurrentPath())) && v.getInitializer() != null) {
                        assigned[0]++;
                        ok[0] &= isNewCollection(v.getInitializer(), fresh, new TreePath(getCurrentPath(), v.getInitializer()));
                    }
                    return super.visitVariable(v, x);
                }

                @Override
                public Void visitIdentifier(IdentifierTree id, Void x) {
                    use(id);
                    return super.visitIdentifier(id, x);
                }

                @Override
                public Void visitMemberSelect(MemberSelectTree ms, Void x) {
                    use(ms);
                    return super.visitMemberSelect(ms, x);
                }

                private void use(Tree t) {
                    if (!field.equals(trees.getElement(getCurrentPath()))) {
                        return;
                    }
                    Tree parent = getCurrentPath().getParentPath().getLeaf();
                    if (parent instanceof MemberSelectTree && ((MemberSelectTree) parent).getExpression() == t
                        || parent instanceof EnhancedForLoopTree && ((EnhancedForLoopTree) parent).getExpression() == t) {
                        return;
                    }
                    if (parent instanceof AssignmentTree && ((AssignmentTree) parent).getVariable() == t) {
                        AssignmentTree a = (AssignmentTree) parent;
                        assigned[0]++;
                        ok[0] &= isNewCollection(a.getExpression(), fresh,
                            new TreePath(getCurrentPath().getParentPath(), a.getExpression()));
                        return;
                    }
                    ok[0] = false;
                }
            }.scan(new TreePath(cu), null);
        }
        return ok[0] && assigned[0] > 0;
    }

    /** The local or field an expression names, or null when it computes a value. */
    private Element named(TreePath p) {
        p = unparen(p);
        if (!(p.getLeaf() instanceof IdentifierTree || p.getLeaf() instanceof MemberSelectTree)) {
            return null;
        }
        Element e = trees.getElement(p);
        return e != null && (e.getKind() == ElementKind.LOCAL_VARIABLE || e.getKind() == ElementKind.FIELD
            || e.getKind() == ElementKind.PARAMETER) ? e : null;
    }

    /** Whether {@code e} is {@code new C<>()} for one of {@code classes}, without arguments or a body. */
    private boolean isNewCollection(ExpressionTree e, Set<String> classes, TreePath p) {
        if (!(e instanceof NewClassTree)) {
            return false;
        }
        NewClassTree nc = (NewClassTree) e;
        return nc.getArguments().isEmpty() && nc.getClassBody() == null
            && classes.contains(qname(types.erasure(trees.getTypeMirror(p))));
    }

    /** {@code list.add(x)}, {@code list.set(i, x)} and {@code map.put(k, v)} as statements. */
    private List<Object> collectionWrite(TreePath ep, Ctx ctx, String pos) {
        MethodInvocationTree mi = (MethodInvocationTree) ep.getLeaf();
        if (!(mi.getMethodSelect() instanceof MemberSelectTree)) {
            return null;
        }
        ExecutableElement m = (ExecutableElement) trees.getElement(ep);
        TypeElement owner = (TypeElement) m.getEnclosingElement();
        if (!owner.getQualifiedName().toString().startsWith("java.util.")) {
            return null;
        }
        String name = m.getSimpleName().toString();
        TreePath sel = new TreePath(ep, mi.getMethodSelect());
        TreePath recvPath = new TreePath(sel, ((MemberSelectTree) mi.getMethodSelect()).getExpression());
        int n = mi.getArguments().size();
        boolean list = name.equals("add") && n == 1 || name.equals("set") && n == 2;
        boolean map = name.equals("put") && n == 2;
        if (!list && !map) {
            return null;
        }
        refuseUnassigned(recvPath, "written by " + name, "a collection write through the field's pointer is outside the reader's subset");
        for (ExpressionTree a : mi.getArguments()) {
            refuseUnassigned(new TreePath(ep, a), "an argument of " + name, "Java stores the null" + STOPS);
        }
        if (!owned(recvPath, ctx)) {
            throw new Unsupported(name + " on a collection another name may share: a Java collection is one object every"
                + " name sees changed, a cir/1 slice or map write is not; only a field of this or a local built here and"
                + " never handed on is written");
        }
        Element target = named(recvPath);
        if (target != null && ctx.iterating.contains(target)) {
            throw new Unsupported(name + " on the collection a for-each is iterating: Java throws"
                + " ConcurrentModificationException, a cir/1 range runs on");
        }
        Map<String, Object> recv = expr(recvPath, ctx);
        Map<String, Object> rt = type(recv);
        if (list && "slice".equals(rt.get("k"))) {
            Map<String, Object> elem = sub(rt, "elem");
            if (name.equals("add")) {
                Map<String, Object> place = place(recvPath, ctx);
                Map<String, Object> x = coerce(expr(new TreePath(ep, mi.getArguments().get(0)), ctx), elem);
                return List.of(assign("", place, intrinsic("append", List.of(recv, x), rt), pos));
            }
            Map<String, Object> i = coerce(expr(new TreePath(ep, mi.getArguments().get(0)), ctx), tInt("32"));
            Map<String, Object> x = coerce(expr(new TreePath(ep, mi.getArguments().get(1)), ctx), elem);
            return List.of(assign("", index(recv, i, elem), x, pos));
        }
        if (map && "map".equals(rt.get("k"))) {
            Map<String, Object> k = coerce(expr(new TreePath(ep, mi.getArguments().get(0)), ctx), sub(rt, "key"));
            Map<String, Object> v = coerce(expr(new TreePath(ep, mi.getArguments().get(1)), ctx), sub(rt, "elem"));
            return List.of(assign("", index(recv, k, sub(rt, "elem")), v, pos));
        }
        return null;
    }

    private List<Object> forLoop(TreePath p, Ctx ctx, String pos) {
        ForLoopTree ft = (ForLoopTree) p.getLeaf();
        Map<String, Object> st = Json.obj();
        st.put("k", "for");
        if (ft.getInitializer().size() > 1 || ft.getUpdate().size() > 1) {
            throw new Unsupported("for with more than one init or update statement");
        }
        if (!ft.getInitializer().isEmpty()) {
            List<Object> init = strict(new TreePath(p, ft.getInitializer().get(0)), ctx);
            String k = init.size() == 1 ? (String) ((Map<?, ?>) init.get(0)).get("k") : "";
            if (!k.equals("let") && !k.equals("assign")) {
                throw new Unsupported("for with an init statement other than a declaration or an assignment");
            }
            st.put("init", init.get(0));
        }
        if (ft.getCondition() != null) {
            Map<String, Object> cond = coerce(expr(new TreePath(p, ft.getCondition()), ctx), tBool());
            if (!isTrue(cond)) {
                st.put("cond", cond);
            }
        }
        Object post = null;
        if (!ft.getUpdate().isEmpty()) {
            List<Object> u = strict(new TreePath(p, ft.getUpdate().get(0)), ctx);
            String k = u.size() == 1 ? (String) ((Map<?, ?>) u.get(0)).get("k") : "";
            if (!k.equals("assign") && !k.equals("incr") && !k.equals("decr")) {
                throw new Unsupported("for with an update other than an assignment, ++ or --");
            }
            post = u.get(0);
        }
        if (post != null) {
            st.put("post", post);
        }
        st.put("body", inside("loop", p, ft.getStatement(), ctx));
        st.put("pos", pos);
        return List.of(st);
    }

    private List<Object> forEach(TreePath p, Ctx ctx, String pos) {
        EnhancedForLoopTree ft = (EnhancedForLoopTree) p.getLeaf();
        String name = ft.getVariable().getName().toString();
        Map<String, Object> vt = typeOf(trees.getElement(new TreePath(p, ft.getVariable())).asType());
        TreePath ep = new TreePath(p, ft.getExpression());
        ExpressionTree e = ft.getExpression();
        String key = "_";
        String val = name;
        Map<String, Object> over;
        Element source;
        if (e instanceof MethodInvocationTree && ((MethodInvocationTree) e).getMethodSelect() instanceof MemberSelectTree
            && ((MemberSelectTree) ((MethodInvocationTree) e).getMethodSelect()).getIdentifier().contentEquals("keySet")
            && ((MethodInvocationTree) e).getArguments().isEmpty()) {
            MemberSelectTree ms = (MemberSelectTree) ((MethodInvocationTree) e).getMethodSelect();
            TreePath mp = new TreePath(new TreePath(ep, ms), ms.getExpression());
            source = named(mp);
            String cls = qname(types.erasure(trees.getTypeMirror(mp)));
            Map<String, Object> m = expr(mp, ctx);
            if (!"map".equals(type(m).get("k"))) {
                throw new Unsupported("keySet of something other than a map");
            }
            if (!sub(type(m), "key").equals(vt)) {
                throw new Unsupported("a loop variable of a type other than the key's");
            }
            if (cls.equals("java.util.HashMap")) {
                key = name;
                val = "_";
                over = m;
            } else if (cls.equals("java.util.TreeMap")) {
                Map<String, Object> kt = sub(type(m), "key");
                if (!kt.equals(tString()) && !isInt(kt)) {
                    throw new Unsupported("a TreeMap over " + describe(kt) + " keys iterates in their compareTo order;"
                        + " sorted_keys orders only strings and integers");
                }
                if (!naturalTreeMap(mp)) {
                    throw new Unsupported("a TreeMap the reader did not see built with new TreeMap<>() may carry any"
                        + " comparator; sorted_keys is the natural order");
                }
                over = intrinsic("sorted_keys", List.of(m), tSlice(kt));
            } else {
                throw new Unsupported("iteration over a " + cls + ": its order depends on the class behind it; declare it HashMap (unordered) or TreeMap (sorted)");
            }
        } else {
            over = expr(ep, ctx);
            source = named(ep);
            String cls = qname(types.erasure(trees.getTypeMirror(ep)));
            if (!"slice".equals(type(over).get("k")) || !(cls.equals("java.util.List") || cls.equals("java.util.ArrayList"))) {
                throw new Unsupported("for-each over something other than a List or a map's keySet()");
            }
            if (!sub(type(over), "elem").equals(vt)) {
                throw new Unsupported("a loop variable of a type other than the element's");
            }
        }
        Map<String, Object> st = Json.obj();
        st.put("k", "for_range");
        st.put("key", key);
        st.put("val", val);
        st.put("over", over);
        if (source != null) {
            ctx.iterating.push(source);
        }
        try {
            st.put("body", inside("loop", p, ft.getStatement(), ctx));
        } finally {
            if (source != null) {
                ctx.iterating.pop();
            }
        }
        st.put("pos", pos);
        return List.of(st);
    }

    private List<Object> switchStmt(TreePath p, Ctx ctx, String pos) {
        SwitchTree sw = (SwitchTree) p.getLeaf();
        Map<String, Object> tag = expr(new TreePath(p, sw.getExpression()), ctx);
        Map<String, Object> tt = type(tag);
        if (!isInt(tt) && !tt.equals(tString()) && !(enumName(tt) != null)) {
            throw new Unsupported("switch on a " + describe(tt));
        }
        List<List<Object>> values = new ArrayList<>();
        List<List<? extends StatementTree>> bodies = new ArrayList<>();
        List<TreePath> paths = new ArrayList<>();
        List<Object> pending = new ArrayList<>();
        boolean pendingDefault = false;
        List<? extends CaseTree> cases = sw.getCases();
        for (int i = 0; i < cases.size(); i++) {
            CaseTree c = cases.get(i);
            TreePath cp = new TreePath(p, c);
            if (c.getCaseKind() != CaseTree.CaseKind.STATEMENT) {
                throw new Unsupported("switch with -> cases");
            }
            boolean isDefault = c.getExpressions().isEmpty();
            if (isDefault && !pending.isEmpty() || !isDefault && pendingDefault) {
                throw new Unsupported("a case label sharing its body with default");
            }
            pendingDefault |= isDefault;
            for (ExpressionTree v : c.getExpressions()) {
                pending.add(coerce(expr(new TreePath(cp, v), ctx), tt));
            }
            List<? extends StatementTree> stmts = c.getStatements();
            boolean last = i == cases.size() - 1;
            if (stmts.isEmpty() && !last) {
                continue;
            }
            if (!stmts.isEmpty()) {
                StatementTree end = stmts.get(stmts.size() - 1);
                if (end instanceof BreakTree && ((BreakTree) end).getLabel() == null) {
                    stmts = stmts.subList(0, stmts.size() - 1);
                } else if (!exits(end) && !(end instanceof ContinueTree) && !last) {
                    throw new Unsupported("a case falling through to the next is outside cir/1");
                }
            }
            values.add(pending);
            bodies.add(stmts);
            paths.add(cp);
            pending = new ArrayList<>();
            pendingDefault = false;
        }
        List<Object> outCases = new ArrayList<>();
        for (int i = 0; i < values.size(); i++) {
            ctx.breaks.push("switch");
            try {
                Map<String, Object> c = Json.obj();
                c.put("values", values.get(i));
                c.put("body", block(paths.get(i), bodies.get(i), ctx));
                outCases.add(c);
            } finally {
                ctx.breaks.pop();
            }
        }
        Map<String, Object> st = Json.obj();
        st.put("k", "switch");
        st.put("tag", tag);
        st.put("cases", outCases);
        st.put("pos", pos);
        return List.of(st);
    }

    private List<Object> returnStmt(TreePath p, Ctx ctx, String pos) {
        ReturnTree rt = (ReturnTree) p.getLeaf();
        Fn fn = ctx.fn;
        if (rt.getExpression() == null) {
            return List.of(ret(successResults(ctx, null), pos));
        }
        TreePath ep = new TreePath(p, rt.getExpression());
        if (fn.optional) {
            ExpressionTree e = rt.getExpression();
            if (e instanceof MethodInvocationTree) {
                ExecutableElement m = (ExecutableElement) trees.getElement(ep);
                String owner = ((TypeElement) m.getEnclosingElement()).getQualifiedName().toString();
                List<? extends ExpressionTree> args = ((MethodInvocationTree) e).getArguments();
                if (owner.equals("java.util.Optional") && m.getSimpleName().contentEquals("empty") && args.isEmpty()) {
                    return List.of(ret(List.of(zero(fn.values.get(0)), lit("false", tBool())), pos));
                }
                if (owner.equals("java.util.Optional") && m.getSimpleName().contentEquals("of") && args.size() == 1) {
                    Map<String, Object> x = coerce(expr(new TreePath(ep, args.get(0)), ctx), fn.values.get(0));
                    return List.of(ret(List.of(x, lit("true", tBool())), pos));
                }
            }
            throw new Unsupported("an Optional other than Optional.of(x) or Optional.empty()");
        }
        if (throwingCallee(ep) != null) {
            throw new Unsupported("return of a call that throws");
        }
        Map<String, Object> x = coerce(expr(ep, ctx), fn.values.get(0));
        return List.of(ret(successResults(ctx, x), pos));
    }

    /** The results of a normal return: the value (or {@code this}), then a nil error. */
    private List<Object> successResults(Ctx ctx, Map<String, Object> value) {
        List<Object> rs = new ArrayList<>();
        if (ctx.fn.isCtor) {
            rs.add(varRef("this", tPtr(tNamed(ctx.fn.cls.getSimpleName().toString()))));
        } else if (value != null) {
            rs.add(value);
        }
        if (ctx.fn.throwsErr) {
            rs.add(nil(tError()));
        }
        return rs;
    }

    private Map<String, Object> ret(List<?> results, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", "return");
        st.put("results", new ArrayList<Object>(results));
        st.put("pos", pos);
        return st;
    }

    private List<Object> throwStmt(TreePath p, Ctx ctx, String pos) {
        ExpressionTree e = ((ThrowTree) p.getLeaf()).getExpression();
        if (!(e instanceof NewClassTree)) {
            throw new Unsupported("throw of something other than new <exception>()");
        }
        NewClassTree nc = (NewClassTree) e;
        TypeElement te = (TypeElement) types.asElement(trees.getTypeMirror(new TreePath(p, e)));
        if (!errorLeaves.containsKey(te) || !nc.getArguments().isEmpty() || nc.getClassBody() != null) {
            throw new Unsupported("throw of an exception outside the package's checked-exception hierarchy");
        }
        if (!ctx.fn.throwsErr) {
            throw new Unsupported("throw in a method without an error result");
        }
        List<Object> rs = new ArrayList<>();
        for (Map<String, Object> t : ctx.fn.values) {
            rs.add(zero(t));
        }
        Map<String, Object> ev = Json.obj();
        ev.put("k", "errvar");
        ev.put("name", te.getSimpleName().toString());
        ev.put("t", tError());
        rs.add(ev);
        return List.of(ret(rs, pos));
    }

    /** The package method a call that throws the package's exceptions invokes, or null. */
    private Fn throwingCallee(TreePath p) {
        Tree t = p.getLeaf();
        if (!(t instanceof MethodInvocationTree) && !(t instanceof NewClassTree)) {
            return null;
        }
        Fn fn = calleeOf(p);
        return fn != null && fn.throwsErr ? fn : null;
    }

    private Fn calleeOf(TreePath p) {
        Element e = trees.getElement(p);
        if (p.getLeaf() instanceof NewClassTree) {
            TypeElement te = (TypeElement) types.asElement(trees.getTypeMirror(p));
            return structs.contains(te) ? ctors.get(te) : null;
        }
        if (!(e instanceof ExecutableElement)) {
            return null;
        }
        Fn fn = fns.get(e);
        if (fn != null) {
            return fn;
        }
        TypeElement owner = (TypeElement) e.getEnclosingElement();
        TypeElement impl = implOf.get(owner);
        if (impl == null) {
            return null;
        }
        for (Map.Entry<ExecutableElement, Fn> en : fns.entrySet()) {
            if (en.getValue().cls.equals(impl) && elements.overrides(en.getKey(), (ExecutableElement) e, impl)) {
                return en.getValue();
            }
        }
        return null;
    }

    /** {@code names := call; if err != nil { return zeros, err }}. */
    private List<Object> propagate(Map<String, Object> call, String valueName, Fn callee, Ctx ctx, String pos) {
        if (!ctx.fn.throwsErr) {
            throw new Unsupported("a call that throws, in a method without an error result");
        }
        String err = ctx.fresh("err");
        List<Object> names = new ArrayList<>();
        if (!callee.values.isEmpty()) {
            names.add(valueName == null ? "_" : valueName);
        }
        names.add(err);
        Map<String, Object> let = Json.obj();
        let.put("k", "let");
        let.put("names", names);
        let.put("rhs", new ArrayList<Object>(List.of(call)));
        let.put("pos", pos);
        List<Object> rs = new ArrayList<>();
        if (ctx.fn.isCtor) {
            rs.add(zero(ctx.fn.values.get(0)));
        } else {
            for (Map<String, Object> t : ctx.fn.values) {
                rs.add(zero(t));
            }
        }
        rs.add(varRef(err, tError()));
        Map<String, Object> iff = Json.obj();
        iff.put("k", "if");
        iff.put("cond", binop("!=", varRef(err, tError()), nil(tError()), tBool()));
        iff.put("then", new ArrayList<Object>(List.of(ret(rs, pos))));
        iff.put("pos", pos);
        return List.of(let, iff);
    }

    // ---- expressions ----

    private Map<String, Object> expr(TreePath p, Ctx ctx) {
        ExpressionTree e = (ExpressionTree) p.getLeaf();
        switch (e.getKind()) {
            case PARENTHESIZED:
                return expr(new TreePath(p, ((ParenthesizedTree) e).getExpression()), ctx);
            case INT_LITERAL:
                return lit(String.valueOf(((LiteralTree) e).getValue()), tInt("32"));
            case LONG_LITERAL:
                return lit(String.valueOf(((LiteralTree) e).getValue()), tInt("64"));
            case BOOLEAN_LITERAL:
                return lit(String.valueOf(((LiteralTree) e).getValue()), tBool());
            case STRING_LITERAL:
                return lit(quote((String) ((LiteralTree) e).getValue()), tString());
            case NULL_LITERAL:
                return nil(NULL);
            case CHAR_LITERAL:
            case FLOAT_LITERAL:
            case DOUBLE_LITERAL:
                throw new Unsupported("char and floating-point values are outside the reader's types");
            case IDENTIFIER:
                return ident(p, ctx);
            case MEMBER_SELECT:
                return select(p, ctx);
            case METHOD_INVOCATION:
            case NEW_CLASS:
                return call(p, ctx, false);
            case PLUS:
            case MINUS:
            case MULTIPLY:
            case DIVIDE:
            case REMAINDER:
            case LESS_THAN:
            case GREATER_THAN:
            case LESS_THAN_EQUAL:
            case GREATER_THAN_EQUAL:
            case EQUAL_TO:
            case NOT_EQUAL_TO:
            case CONDITIONAL_AND:
            case CONDITIONAL_OR:
            case AND:
            case OR:
            case XOR:
                return binary(p, ctx);
            case LEFT_SHIFT:
            case RIGHT_SHIFT:
            case UNSIGNED_RIGHT_SHIFT:
                throw new Unsupported("shift: Java masks the shift count, cir/1 does not say so");
            case UNARY_MINUS:
            case UNARY_PLUS:
            case LOGICAL_COMPLEMENT:
            case BITWISE_COMPLEMENT: {
                refuseUnassigned(new TreePath(p, ((UnaryTree) e).getExpression()), "an operand of a unary operator", "Java unboxes the null" + STOPS);
                Map<String, Object> t = typeOf(trees.getTypeMirror(p));
                Map<String, Object> x = coerce(expr(new TreePath(p, ((UnaryTree) e).getExpression()), ctx), t);
                String op = e.getKind() == Tree.Kind.UNARY_MINUS ? "-"
                    : e.getKind() == Tree.Kind.UNARY_PLUS ? "+" : e.getKind() == Tree.Kind.LOGICAL_COMPLEMENT ? "!" : "^";
                return unop(op, x, t);
            }
            case LAMBDA_EXPRESSION:
                throw new Unsupported("lambda: functions as values are outside cir/1");
            case MEMBER_REFERENCE:
                throw new Unsupported("method reference: functions as values are outside cir/1");
            case CONDITIONAL_EXPRESSION:
                throw new Unsupported("?: is outside cir/1");
            case INSTANCE_OF:
                throw new Unsupported("instanceof: runtime types are outside cir/1");
            case TYPE_CAST:
                throw new Unsupported("cast: conversions are outside the reader's subset");
            case ASSIGNMENT:
                throw new Unsupported("an assignment inside an expression");
            case POSTFIX_INCREMENT:
            case PREFIX_INCREMENT:
            case POSTFIX_DECREMENT:
            case PREFIX_DECREMENT:
                throw new Unsupported("++ or -- inside an expression");
            case ARRAY_ACCESS:
            case NEW_ARRAY:
                throw new Unsupported("arrays are outside the reader's types");
            default:
                throw new Unsupported(e.getKind().toString().toLowerCase(Locale.ROOT).replace('_', ' ') + " is outside cir/1");
        }
    }

    private Map<String, Object> thisRef(Ctx ctx) {
        if (!ctx.hasThis) {
            throw new Unsupported("this where there is no object: a static method or a field initializer");
        }
        return varRef("this", tPtr(tNamed(ctx.fn.cls.getSimpleName().toString())));
    }

    private Map<String, Object> ident(TreePath p, Ctx ctx) {
        IdentifierTree id = (IdentifierTree) p.getLeaf();
        if (id.getName().contentEquals("this")) {
            return thisRef(ctx);
        }
        if (id.getName().contentEquals("super")) {
            throw new Unsupported("super: inheritance is outside cir/1");
        }
        Element el = trees.getElement(p);
        switch (el.getKind()) {
            case LOCAL_VARIABLE:
            case PARAMETER:
            case EXCEPTION_PARAMETER:
            case RESOURCE_VARIABLE:
                return varRef(id.getName().toString(), typeOf(trees.getTypeMirror(p)));
            case FIELD:
            case ENUM_CONSTANT:
                return fieldRef((VariableElement) el, null, ctx);
            default:
                throw new Unsupported(el.getKind().toString().toLowerCase(Locale.ROOT) + " " + id.getName() + " as a value");
        }
    }

    private Map<String, Object> select(TreePath p, Ctx ctx) {
        MemberSelectTree ms = (MemberSelectTree) p.getLeaf();
        if (ms.getIdentifier().contentEquals("class")) {
            throw new Unsupported("class literal: reflection is outside cir/1");
        }
        Element el = trees.getElement(p);
        if (el == null || el.getKind() != ElementKind.FIELD && el.getKind() != ElementKind.ENUM_CONSTANT) {
            throw new Unsupported(ms.getIdentifier() + " is outside the reader's subset");
        }
        if (el.getModifiers().contains(Modifier.STATIC)) {
            if (!namesAType(new TreePath(p, ms.getExpression()))) {
                throw new Unsupported("a static field read through an expression, which Java evaluates and then ignores");
            }
            return fieldRef((VariableElement) el, null, ctx);
        }
        return fieldRef((VariableElement) el, expr(new TreePath(p, ms.getExpression()), ctx), ctx);
    }

    private Map<String, Object> fieldRef(VariableElement el, Map<String, Object> recv, Ctx ctx) {
        TypeElement owner = (TypeElement) el.getEnclosingElement();
        String name = el.getSimpleName().toString();
        if (el.getKind() == ElementKind.ENUM_CONSTANT) {
            if (!enums.contains(owner)) {
                throw new Unsupported("constant of enum " + owner.getQualifiedName() + ", which is not read");
            }
            return constRef(name, tNamed(owner.getSimpleName().toString()));
        }
        if (el.getModifiers().contains(Modifier.STATIC)) {
            if (consts.containsKey(el)) {
                return constRef(consts.get(el), typeOf(el.asType()));
            }
            String on = owner.getQualifiedName().toString();
            if ((on.equals("java.lang.Integer") || on.equals("java.lang.Long")) && el.getConstantValue() != null) {
                return constLit(el.getConstantValue(), typeOf(el.asType()));
            }
            throw new Unsupported("static field " + owner.getSimpleName() + "." + name);
        }
        if (!structs.contains(owner)) {
            throw new Unsupported("field of " + owner.getQualifiedName() + ", which is not read");
        }
        Map<String, Object> x = recv != null ? recv : thisRef(ctx);
        Map<String, Object> f = Json.obj();
        f.put("k", "field");
        f.put("x", x);
        f.put("name", name);
        f.put("t", fieldType(el));
        return derefed(el) ? unop("*", f, typeOf(el.asType())) : f;
    }

    /**
     * The cir/1 type of an instance field: its mapped type, or for a field that holds null until
     * assigned, {@link JvmTypes#unassignedField} of it, a pointer whose zero value is nil.
     */
    private Map<String, Object> fieldType(VariableElement el) {
        Map<String, Object> t = typeOf(el.asType());
        return unassigned.contains(el) ? JvmTypes.unassignedField(t) : t;
    }

    /**
     * Whether a field holds null until assigned and its value is read through a pointer: a String,
     * box, list, map or enum field; not a class of the package, whose value is a pointer already.
     */
    private boolean derefed(VariableElement el) {
        return unassigned.contains(el) && !fieldType(el).equals(typeOf(el.asType()));
    }

    /**
     * The field that holds null until assigned which {@code p} reads through a pointer
     * ({@link #derefed}), or null when {@code p} reads anything else.
     */
    private VariableElement unassignedRead(TreePath p) {
        p = unparen(p);
        if (!(p.getLeaf() instanceof IdentifierTree || p.getLeaf() instanceof MemberSelectTree)) {
            return null;
        }
        Element e = trees.getElement(p);
        return e instanceof VariableElement && e.getKind() == ElementKind.FIELD && derefed((VariableElement) e)
            ? (VariableElement) e : null;
    }

    /** The pointer itself of a field {@link #unassignedRead} names: its read without the dereference. */
    private Map<String, Object> unassignedPointer(TreePath p, Ctx ctx) {
        Map<String, Object> d = expr(p, ctx);
        return sub(d, "x");
    }

    /** How the written program differs where Java takes a null: it dereferences the nil pointer and panics. */
    private static final String STOPS = ", where the written program stops at the nil pointer";

    /**
     * Refuse {@code p} when it reads a field that holds null until assigned: {@code use} is what the
     * expression does with it, {@code why} why that is not read.
     */
    private void refuseUnassigned(TreePath p, String use, String why) {
        VariableElement f = unassignedRead(p);
        if (f != null) {
            throw new Unsupported("field " + f.getSimpleName() + ", null until assigned, " + use + ": " + why);
        }
    }

    private Map<String, Object> constLit(Object v, Map<String, Object> t) {
        if (v instanceof String) {
            return lit(quote((String) v), t);
        }
        if (v instanceof Boolean || v instanceof Integer || v instanceof Long || v instanceof Short || v instanceof Byte) {
            return lit(String.valueOf(v), t);
        }
        throw new Unsupported("constant of type " + v.getClass().getSimpleName());
    }

    /** A method call or {@code new}; with {@code mayThrow}, also one of a method that throws. */
    private Map<String, Object> call(TreePath p, Ctx ctx, boolean mayThrow) {
        Tree t = p.getLeaf();
        if (t instanceof NewClassTree) {
            return newClass(p, ctx, mayThrow);
        }
        MethodInvocationTree mi = (MethodInvocationTree) t;
        if (!mi.getTypeArguments().isEmpty()) {
            throw new Unsupported("explicit type arguments: generics are outside cir/1");
        }
        ExecutableElement m = (ExecutableElement) trees.getElement(p);
        TypeElement owner = (TypeElement) m.getEnclosingElement();
        String on = owner.getQualifiedName().toString();
        String name = m.getSimpleName().toString();
        if (on.equals("java.lang.Class") || on.startsWith("java.lang.reflect.") || name.equals("getClass")) {
            throw new Unsupported("reflection: " + owner.getSimpleName() + "." + name + " is outside cir/1");
        }
        if (on.startsWith("java.util.stream.") || (on.startsWith("java.util.") && (name.equals("stream") || name.equals("parallelStream")))) {
            throw new Unsupported("stream: " + owner.getSimpleName() + "." + name + " is outside cir/1");
        }
        if (on.startsWith("java.util.function.")) {
            throw new Unsupported("call through a functional interface: lambdas are outside cir/1");
        }
        ExpressionTree sel = mi.getMethodSelect();
        TreePath selPath = new TreePath(p, sel);
        TreePath recvPath = sel instanceof MemberSelectTree ? new TreePath(selPath, ((MemberSelectTree) sel).getExpression()) : null;
        if (recvPath != null && recvPath.getLeaf() instanceof IdentifierTree
            && ((IdentifierTree) recvPath.getLeaf()).getName().contentEquals("super")) {
            throw new Unsupported("super: inheritance is outside cir/1");
        }
        if (m.getModifiers().contains(Modifier.STATIC) && recvPath != null && !namesAType(recvPath)) {
            throw new Unsupported("a static method called through an expression, which Java evaluates and then ignores");
        }
        Fn fn = calleeOf(p);
        if (fn != null) {
            if (fn.foreignThrows != null) {
                throw new Unsupported("call of " + fn.key() + ", which throws " + fn.foreignThrows);
            }
            if (fn.throwsErr && !mayThrow) {
                throw new Unsupported("a call that throws inside an expression: only a statement or a local's initializer checks the error");
            }
            if (fn.optional) {
                throw new Unsupported("use of an Optional result");
            }
            List<Object> args = args(p, mi.getArguments(), fn.params, ctx);
            Map<String, Object> c = Json.obj();
            if (fn.isStatic) {
                c.put("k", "call");
                c.put("name", fn.name);
            } else {
                c.put("k", "method");
                c.put("x", recvPath == null ? thisRef(ctx) : expr(recvPath, ctx));
                c.put("name", fn.name);
            }
            c.put("args", args);
            c.put("t", resultType(fn));
            return c;
        }
        if (inPackage(owner)) {
            throw new Unsupported("call of " + owner.getSimpleName() + "." + name + ", which is not read");
        }
        for (ExpressionTree a : mi.getArguments()) {
            refuseUnassigned(new TreePath(p, a), "an argument of " + owner.getSimpleName() + "." + name,
                "Java hands the null to the library method, which prints, stores or compares it" + STOPS);
        }
        int n = mi.getArguments().size();
        if (on.equals("java.lang.String") && name.equals("format") && m.getModifiers().contains(Modifier.STATIC)) {
            return format(p, mi, ctx);
        }
        if (recvPath == null || !on.startsWith("java.")) {
            throw new Unsupported(owner.getSimpleName() + "." + name + " is outside the reader's library table");
        }
        Map<String, Object> recv = expr(recvPath, ctx);
        Map<String, Object> rt = type(recv);
        String rk = (String) rt.get("k");
        if ("slice".equals(rk) && on.startsWith("java.util.")) {
            if (name.equals("size") && n == 0) {
                return size(recv);
            }
            if (name.equals("isEmpty") && n == 0) {
                return binop("==", intrinsic("len", List.of(recv), tInt("platform")), lit("0", tInt("platform")), tBool());
            }
            if (name.equals("get") && n == 1) {
                Map<String, Object> i = coerce(expr(new TreePath(p, mi.getArguments().get(0)), ctx), tInt("32"));
                return index(recv, i, sub(rt, "elem"));
            }
        }
        if ("map".equals(rk) && on.startsWith("java.util.")) {
            Map<String, Object> elem = sub(rt, "elem");
            if (name.equals("size") && n == 0) {
                return size(recv);
            }
            if (name.equals("get") && n == 1) {
                if (!"ptr".equals(elem.get("k"))) {
                    throw new Unsupported("Map.get of a value that is not a reference: an absent key gives null, not a zero value");
                }
                return index(recv, coerce(expr(new TreePath(p, mi.getArguments().get(0)), ctx), sub(rt, "key")), elem);
            }
            if (name.equals("getOrDefault") && n == 2) {
                Map<String, Object> d = coerce(expr(new TreePath(p, mi.getArguments().get(1)), ctx), elem);
                if (!isZeroLit(d)) {
                    throw new Unsupported("getOrDefault with a default other than the zero value");
                }
                return index(recv, coerce(expr(new TreePath(p, mi.getArguments().get(0)), ctx), sub(rt, "key")), elem);
            }
        }
        if (rt.equals(tString()) && on.equals("java.lang.String")) {
            if (name.equals("equals") && n == 1) {
                Map<String, Object> y = expr(new TreePath(p, mi.getArguments().get(0)), ctx);
                if (!type(y).equals(tString())) {
                    throw new Unsupported("String.equals of something other than a String");
                }
                return binop("==", recv, y, tBool());
            }
            if (name.equals("isEmpty") && n == 0) {
                return binop("==", recv, lit("\"\"", tString()), tBool());
            }
        }
        throw new Unsupported(owner.getSimpleName() + "." + name + " is outside the reader's library table");
    }

    /**
     * {@code String.format} as {@code sprintf}, for the formats java.util.Formatter and cir/1's
     * {@code sprintf} print alike: a literal format whose only conversions are {@code %s} of a
     * String, {@code %d} of an integer and {@code %%}, without flags, width, precision or index,
     * one argument per conversion.
     */
    private Map<String, Object> format(TreePath p, MethodInvocationTree mi, Ctx ctx) {
        List<? extends ExpressionTree> as = mi.getArguments();
        if (as.isEmpty() || as.get(0).getKind() != Tree.Kind.STRING_LITERAL) {
            throw new Unsupported("String.format of a format that is not a literal: its conversions are unknown");
        }
        String f = (String) ((LiteralTree) as.get(0)).getValue();
        List<Character> verbs = new ArrayList<>();
        for (int i = 0; i < f.length(); i++) {
            if (f.charAt(i) != '%') {
                continue;
            }
            char v = i + 1 < f.length() ? f.charAt(i + 1) : ' ';
            if (v != 's' && v != 'd' && v != '%') {
                throw new Unsupported("String.format conversion " + f.substring(i, Math.min(i + 2, f.length()))
                    + ": only %s, %d and %% print as cir/1's sprintf prints them");
            }
            if (v != '%') {
                verbs.add(v);
            }
            i++;
        }
        if (verbs.size() != as.size() - 1) {
            throw new Unsupported("String.format with " + verbs.size() + " conversions and " + (as.size() - 1) + " arguments");
        }
        List<Object> args = new ArrayList<>();
        args.add(expr(new TreePath(p, as.get(0)), ctx));
        for (int i = 1; i < as.size(); i++) {
            TreePath ap = new TreePath(p, as.get(i));
            refusePrintedNullable(ap, ctx);
            if (trees.getTypeMirror(ap).getKind() == TypeKind.ARRAY) {
                throw new Unsupported("String.format with an array of arguments");
            }
            Map<String, Object> x = expr(ap, ctx);
            Map<String, Object> t = type(x);
            boolean fits = verbs.get(i - 1) == 's' ? t.equals(tString()) : isInt(t);
            if (!fits) {
                throw new Unsupported("String.format %" + verbs.get(i - 1) + " of a " + describe(t)
                    + ": only %s of a String and %d of an integer print alike");
            }
            args.add(x);
        }
        return intrinsic("sprintf", args, tString());
    }

    /**
     * {@code size()}: {@code len}, of the platform int as the Go reader types it, converted to
     * {@code int32}, the type Java gives {@code size()}.
     */
    private static Map<String, Object> size(Map<String, Object> recv) {
        Map<String, Object> c = kind("conv");
        c.put("name", "int32");
        c.put("x", intrinsic("len", List.of(recv), tInt("platform")));
        c.put("t", tInt("32"));
        return c;
    }

    private static boolean isZeroLit(Map<String, Object> e) {
        if (!"lit".equals(e.get("k"))) {
            return false;
        }
        Object v = e.get("value");
        return "0".equals(v) || "false".equals(v) || "\"\"".equals(v);
    }

    private Map<String, Object> newClass(TreePath p, Ctx ctx, boolean mayThrow) {
        NewClassTree nc = (NewClassTree) p.getLeaf();
        if (nc.getClassBody() != null) {
            throw new Unsupported("anonymous class is outside cir/1");
        }
        TypeMirror tm = trees.getTypeMirror(p);
        TypeElement te = (TypeElement) types.asElement(tm);
        String qn = te.getQualifiedName().toString();
        if (structs.contains(te)) {
            Fn fn = ctors.get(te);
            if (fn.throwsErr && !mayThrow) {
                throw new Unsupported("a constructor that throws, inside an expression");
            }
            if (fn.foreignThrows != null) {
                throw new Unsupported("constructor of " + te.getSimpleName() + ", which throws " + fn.foreignThrows);
            }
            Map<String, Object> c = Json.obj();
            c.put("k", "call");
            c.put("name", fn.name);
            c.put("args", args(p, nc.getArguments(), fn.params, ctx));
            c.put("t", resultType(fn));
            return c;
        }
        if ((qn.equals("java.util.ArrayList") || qn.equals("java.util.HashMap") || qn.equals("java.util.TreeMap"))
            && nc.getArguments().isEmpty()) {
            Map<String, Object> mk = Json.obj();
            mk.put("k", "make");
            mk.put("t", typeOf(tm));
            return mk;
        }
        if (errorLeaves.containsKey(te)) {
            throw new Unsupported("an exception created other than by throw");
        }
        throw new Unsupported("new " + te.getSimpleName() + " is outside the reader's library table");
    }

    private List<Object> args(TreePath p, List<? extends ExpressionTree> as, List<Map<String, Object>> params, Ctx ctx) {
        List<Object> out = new ArrayList<>();
        for (int i = 0; i < as.size(); i++) {
            out.add(coerce(expr(new TreePath(p, as.get(i)), ctx), params.get(i)));
        }
        return out;
    }

    private static Map<String, Object> resultType(Fn fn) {
        List<Map<String, Object>> rs = fn.results();
        if (rs.isEmpty()) {
            return tUnit();
        }
        if (rs.size() == 1) {
            return rs.get(0);
        }
        Map<String, Object> t = Json.obj();
        t.put("k", "tuple");
        t.put("elems", new ArrayList<Object>(rs));
        return t;
    }

    private Map<String, Object> binary(TreePath p, Ctx ctx) {
        BinaryTree b = (BinaryTree) p.getLeaf();
        Map<String, Object> t = typeOf(trees.getTypeMirror(p));
        if (b.getKind() == Tree.Kind.PLUS && t.equals(tString())) {
            return concat(p, ctx);
        }
        TreePath lp = new TreePath(p, b.getLeftOperand());
        TreePath rp = new TreePath(p, b.getRightOperand());
        // A field that holds null until assigned, against null: whether its pointer is nil. Any other
        // operator unboxes, prints or compares the object Java holds there, which may be null.
        boolean leftNull = unparen(lp).getLeaf().getKind() == Tree.Kind.NULL_LITERAL;
        boolean rightNull = unparen(rp).getLeaf().getKind() == Tree.Kind.NULL_LITERAL;
        if ((b.getKind() == Tree.Kind.EQUAL_TO || b.getKind() == Tree.Kind.NOT_EQUAL_TO) && (leftNull || rightNull)) {
            TreePath fp = leftNull ? rp : lp;
            VariableElement f = unassignedRead(fp);
            if (f != null) {
                return binop(binOp(b.getKind()), unassignedPointer(fp, ctx), nil(fieldType(f)), tBool());
            }
        }
        for (TreePath o : List.of(lp, rp)) {
            refuseUnassigned(o, "an operand of " + binOp(b.getKind()), "Java unboxes the null or compares it as no object" + STOPS);
        }
        Map<String, Object> x = expr(lp, ctx);
        Map<String, Object> y = expr(rp, ctx);
        String op = binOp(b.getKind());
        switch (b.getKind()) {
            case CONDITIONAL_AND:
            case CONDITIONAL_OR:
                return binop(op, coerce(x, tBool()), coerce(y, tBool()), tBool());
            case EQUAL_TO:
            case NOT_EQUAL_TO:
            case LESS_THAN:
            case GREATER_THAN:
            case LESS_THAN_EQUAL:
            case GREATER_THAN_EQUAL: {
                if (b.getKind() == Tree.Kind.EQUAL_TO || b.getKind() == Tree.Kind.NOT_EQUAL_TO) {
                    for (TreePath o : List.of(lp, rp)) {
                        String q = qname(trees.getTypeMirror(o));
                        if (q.equals("java.lang.String") || JvmTypes.isIntegerBox(q)) {
                            throw new Unsupported("== on a String or a boxed number compares references");
                        }
                    }
                    // Against null, == asks whether there is an object, which nil answers. Between two
                    // objects it asks whether they are one, which cir/1 does not say of slices, maps or
                    // structs; only an enum value, which has no identity of its own, compares.
                    boolean againstNull = b.getLeftOperand().getKind() == Tree.Kind.NULL_LITERAL
                        || b.getRightOperand().getKind() == Tree.Kind.NULL_LITERAL;
                    for (TreePath o : List.of(lp, rp)) {
                        TypeMirror ot = trees.getTypeMirror(o);
                        if (!againstNull && (ot.getKind() == TypeKind.DECLARED || ot.getKind() == TypeKind.ARRAY)
                            && types.asElement(ot).getKind() != ElementKind.ENUM) {
                            throw new Unsupported("== on two objects compares their identity, which cir/1 does not express");
                        }
                    }
                }
                if (type(x).equals(NULL)) {
                    x = coerce(x, type(y));
                } else if (type(y).equals(NULL) || !type(x).equals(type(y)) && isIntLit(y)) {
                    y = coerce(y, type(x));
                } else {
                    x = coerce(x, type(y));
                }
                return binop(op, x, y, tBool());
            }
            default:
                if (!isInt(t)) {
                    throw new Unsupported(op + " on a " + describe(t));
                }
                return binop(op, coerce(x, t), coerce(y, t), t);
        }
    }

    private static boolean isIntLit(Map<String, Object> e) {
        return "lit".equals(e.get("k")) && isInt(type(e));
    }

    private static String binOp(Tree.Kind k) {
        switch (k) {
            case PLUS:
                return "+";
            case MINUS:
                return "-";
            case MULTIPLY:
                return "*";
            case DIVIDE:
                return "/";
            case REMAINDER:
                return "%";
            case LESS_THAN:
                return "<";
            case GREATER_THAN:
                return ">";
            case LESS_THAN_EQUAL:
                return "<=";
            case GREATER_THAN_EQUAL:
                return ">=";
            case EQUAL_TO:
                return "==";
            case NOT_EQUAL_TO:
                return "!=";
            case CONDITIONAL_AND:
                return "&&";
            case CONDITIONAL_OR:
                return "||";
            case AND:
                return "&";
            case OR:
                return "|";
            default:
                return "^";
        }
    }

    /**
     * String concatenation: of Strings only, {@code +}; otherwise {@code sprintf} with {@code %s}
     * for a String or enum, {@code %d} for an integer and {@code %t} for a boolean, each printing
     * what Java's conversion prints.
     */
    private Map<String, Object> concat(TreePath p, Ctx ctx) {
        List<TreePath> parts = new ArrayList<>();
        flatten(p, parts);
        List<Map<String, Object>> xs = new ArrayList<>();
        boolean allStrings = true;
        StringBuilder fmt = new StringBuilder();
        for (TreePath part : parts) {
            refusePrintedNullable(part, ctx);
            refuseUnassigned(part, "printed by string concatenation", "Java prints null" + STOPS);
            Map<String, Object> x = expr(part, ctx);
            Map<String, Object> t = type(x);
            if (t.equals(tString())) {
                fmt.append("%s");
            } else if (isInt(t)) {
                fmt.append("%d");
                allStrings = false;
            } else if (t.equals(tBool())) {
                fmt.append("%t");
                allStrings = false;
            } else if (enumName(t) != null) {
                fmt.append("%s");
                allStrings = false;
            } else {
                throw new Unsupported("string concatenation of a " + describe(t));
            }
            xs.add(x);
        }
        if (allStrings) {
            Map<String, Object> acc = xs.get(0);
            for (int i = 1; i < xs.size(); i++) {
                acc = binop("+", acc, xs.get(i), tString());
            }
            return acc;
        }
        List<Object> args = new ArrayList<>();
        args.add(lit(quote(fmt.toString()), tString()));
        args.addAll(xs);
        return intrinsic("sprintf", args, tString());
    }

    private void flatten(TreePath p, List<TreePath> out) {
        Tree t = p.getLeaf();
        if (t.getKind() == Tree.Kind.PLUS && typeOfOrNull(trees.getTypeMirror(p)) != null
            && typeOf(trees.getTypeMirror(p)).equals(tString())) {
            BinaryTree b = (BinaryTree) t;
            flatten(new TreePath(p, b.getLeftOperand()), out);
            flatten(new TreePath(p, b.getRightOperand()), out);
        } else {
            out.add(p);
        }
    }

    // ---- types ----

    private static final Map<String, Object> NULL = Map.of("k", "null");

    private Map<String, Object> typeOfOrNull(TypeMirror t) {
        try {
            return typeOf(t);
        } catch (Unsupported u) {
            return null;
        }
    }

    /** The cir/1 type of {@code t}: the JVM type table of {@link JvmTypes}, with the package's own classes. */
    private Map<String, Object> typeOf(TypeMirror t) {
        if (t.getKind() == TypeKind.NULL) {
            return NULL;
        }
        try {
            return JvmTypes.map(jvmType(t), this::ownType);
        } catch (JvmTypes.Outside o) {
            throw new Unsupported(o.getMessage());
        }
    }

    /** {@code t} as the JVM type table describes it, printed as javac prints it. */
    private static JvmTypes.Type jvmType(TypeMirror t) {
        switch (t.getKind()) {
            case BOOLEAN:
            case BYTE:
            case SHORT:
            case INT:
            case LONG:
            case CHAR:
            case FLOAT:
            case DOUBLE:
                return JvmTypes.Type.primitive(JvmTypes.Primitive.valueOf(t.getKind().name()), t.toString());
            case DECLARED: {
                DeclaredType dt = (DeclaredType) t;
                List<JvmTypes.Type> args = new ArrayList<>();
                for (TypeMirror a : dt.getTypeArguments()) {
                    args.add(jvmType(a));
                }
                return JvmTypes.Type.declared(((TypeElement) dt.asElement()).getQualifiedName().toString(), args, t.toString());
            }
            default:
                return JvmTypes.Type.other(t.toString());
        }
    }

    /**
     * A class of the package: a class as {@code ptr(named)}, an enum as {@code named}, an interface
     * with one implementation as that class, the checked-exception hierarchy as {@code error}; else
     * {@code null}, which the table refuses.
     */
    private Map<String, Object> ownType(JvmTypes.Type t) {
        List<TypeElement> own = new ArrayList<>(structs);
        own.addAll(enums);
        own.addAll(implOf.keySet());
        own.addAll(errorLeaves.keySet());
        if (errorRoot != null) {
            own.add(errorRoot);
        }
        TypeElement te = null;
        for (TypeElement e : own) {
            if (e.getQualifiedName().contentEquals(t.className())) {
                te = e;
                break;
            }
        }
        if (te == null) {
            return null;
        }
        if (structs.contains(te)) {
            return tPtr(tNamed(te.getSimpleName().toString()));
        }
        if (enums.contains(te)) {
            return tNamed(te.getSimpleName().toString());
        }
        if (implOf.containsKey(te)) {
            return tPtr(tNamed(implOf.get(te).getSimpleName().toString()));
        }
        if (te.equals(errorRoot) || errorLeaves.containsKey(te)) {
            return tError();
        }
        return null;
    }

    private String enumName(Map<String, Object> t) {
        if (!"named".equals(t.get("k"))) {
            return null;
        }
        for (TypeElement e : enums) {
            if (e.getSimpleName().contentEquals((String) t.get("name"))) {
                return e.getSimpleName().toString();
            }
        }
        return null;
    }

    private static boolean isInt(Map<String, Object> t) {
        return "int".equals(t.get("k"));
    }

    private static String describe(Map<String, Object> t) {
        return Json.write(t).replaceAll("\\s+", " ").trim();
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> type(Map<?, ?> e) {
        return (Map<String, Object>) e.get("t");
    }

    @SuppressWarnings("unchecked")
    private static Map<String, Object> sub(Map<String, Object> t, String key) {
        return (Map<String, Object>) t.get(key);
    }

    /**
     * {@code e} where a {@code want} is expected: unchanged when the types are equal; {@code null}
     * given the reference type; an integer literal typed as {@code want} when its value fits, as
     * Java's constant conversion does. Every other conversion is outside the subset.
     */
    private Map<String, Object> coerce(Map<String, Object> e, Map<String, Object> want) {
        Map<String, Object> t = type(e);
        if (t.equals(want)) {
            return e;
        }
        if (t.equals(NULL)) {
            String k = (String) want.get("k");
            if (!k.equals("ptr") && !k.equals("error")) {
                throw new Unsupported("null where a " + describe(want) + " is expected");
            }
            return nil(want);
        }
        if ("lit".equals(e.get("k")) && isInt(t) && isInt(want)) {
            long v = Long.parseLong((String) e.get("value"));
            int bits = Integer.parseInt((String) want.get("width"));
            if (bits == 64 || (v >= -(1L << (bits - 1)) && v < (1L << (bits - 1)))) {
                return lit((String) e.get("value"), want);
            }
        }
        throw new Unsupported("a " + describe(t) + " where a " + describe(want) + " is expected: implicit conversions are outside cir/1");
    }

    /** Refuse a statement whose output still holds an untyped null. */
    private static void checkTyped(Object o) {
        if (o instanceof Map) {
            Map<?, ?> m = (Map<?, ?>) o;
            if (NULL.equals(m.get("t"))) {
                throw new Unsupported("null whose type the context does not give");
            }
            for (Object v : m.values()) {
                checkTyped(v);
            }
        } else if (o instanceof List) {
            for (Object v : (List<?>) o) {
                checkTyped(v);
            }
        }
    }

    // ---- node builders ----

    private static Map<String, Object> tInt(String width) {
        Map<String, Object> t = Json.obj();
        t.put("k", "int");
        t.put("width", width);
        t.put("signed", true);
        return t;
    }

    private static Map<String, Object> kind(String k) {
        Map<String, Object> t = Json.obj();
        t.put("k", k);
        return t;
    }

    private static Map<String, Object> tBool() {
        return kind("bool");
    }

    private static Map<String, Object> tString() {
        return kind("string");
    }

    private static Map<String, Object> tError() {
        return kind("error");
    }

    private static Map<String, Object> tUnit() {
        return kind("unit");
    }

    private static Map<String, Object> tNamed(String name) {
        Map<String, Object> t = kind("named");
        t.put("name", name);
        return t;
    }

    private static Map<String, Object> tPtr(Map<String, Object> elem) {
        Map<String, Object> t = kind("ptr");
        t.put("elem", elem);
        return t;
    }

    private static Map<String, Object> tSlice(Map<String, Object> elem) {
        Map<String, Object> t = kind("slice");
        t.put("elem", elem);
        return t;
    }

    private static Map<String, Object> lit(String value, Map<String, Object> t) {
        Map<String, Object> e = kind("lit");
        e.put("value", value);
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> nil(Map<String, Object> t) {
        Map<String, Object> e = kind("nil");
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> zero(Map<String, Object> t) {
        Map<String, Object> e = kind("zero");
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> varRef(String name, Map<String, Object> t) {
        Map<String, Object> e = kind("var");
        e.put("name", name);
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> constRef(String name, Map<String, Object> t) {
        Map<String, Object> e = kind("const");
        e.put("name", name);
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> index(Map<String, Object> x, Map<String, Object> i, Map<String, Object> t) {
        Map<String, Object> e = kind("index");
        e.put("x", x);
        e.put("index", i);
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> binop(String op, Map<String, Object> x, Map<String, Object> y, Map<String, Object> t) {
        Map<String, Object> e = kind("binop");
        e.put("op", op);
        e.put("x", x);
        e.put("y", y);
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> unop(String op, Map<String, Object> x, Map<String, Object> t) {
        Map<String, Object> e = kind("unop");
        e.put("op", op);
        e.put("x", x);
        e.put("t", t);
        return e;
    }

    private static Map<String, Object> intrinsic(String name, List<?> args, Map<String, Object> t) {
        Map<String, Object> e = kind("intrinsic");
        e.put("name", name);
        e.put("args", new ArrayList<Object>(args));
        e.put("t", t);
        return e;
    }

    /** A string literal's value in Go spelling: double quotes, backslash escapes. */
    private static String quote(String s) {
        StringBuilder b = new StringBuilder("\"");
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (Character.isSurrogate(c)) {
                if (Character.isHighSurrogate(c) && i + 1 < s.length() && Character.isLowSurrogate(s.charAt(i + 1))) {
                    b.append(c).append(s.charAt(++i));
                    continue;
                }
                throw new Unsupported("a string with an unpaired surrogate");
            }
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
                case '\t':
                    b.append("\\t");
                    break;
                case '\r':
                    b.append("\\r");
                    break;
                default:
                    if (c < 0x20 || c == 0x7f) {
                        b.append(String.format("\\x%02x", (int) c));
                    } else {
                        b.append(c);
                    }
            }
        }
        return b.append('"').toString();
    }

    // ---- positions, source text and refusals ----

    private String pos(CompilationUnitTree cu, Tree t) {
        return pos(cu, positions.getStartPosition(cu, t));
    }

    private String pos(CompilationUnitTree cu, long offset) {
        return fileName(cu) + ":" + cu.getLineMap().getLineNumber(offset);
    }

    /** A declaration's position: where it starts after its annotations. */
    private String declPos(CompilationUnitTree cu, Tree t, ModifiersTree mods) {
        List<? extends AnnotationTree> anns = mods.getAnnotations();
        if (anns.isEmpty()) {
            return pos(cu, t);
        }
        String src = sources.get(cu);
        int i = (int) positions.getEndPosition(cu, anns.get(anns.size() - 1));
        while (i < src.length() && Character.isWhitespace(src.charAt(i))) {
            i++;
        }
        return pos(cu, i);
    }

    private String source(CompilationUnitTree cu, Tree t) {
        return sources.get(cu).substring((int) positions.getStartPosition(cu, t), (int) positions.getEndPosition(cu, t));
    }

    private static String firstLine(String s) {
        int i = s.indexOf('\n');
        return i < 0 ? s : s.substring(0, i) + " …";
    }

    private void refuse(String pos, String what, String reason) {
        Map<String, Object> r = Json.obj();
        r.put("pos", pos);
        r.put("what", what);
        r.put("reason", reason);
        refusals.add(r);
    }

    private void refuseDecl(CompilationUnitTree cu, Tree t, ModifiersTree mods, String what, String reason) {
        refuse(declPos(cu, t, mods), what, reason);
    }

    private void refuseAnnotations(CompilationUnitTree cu, ModifiersTree mods) {
        for (AnnotationTree a : mods.getAnnotations()) {
            TypeMirror at = trees.getTypeMirror(new TreePath(new TreePath(cu), a.getAnnotationType()));
            String qn = at == null ? "" : qname(at);
            if (!HARMLESS_ANNOTATIONS.contains(qn)) {
                refuse(pos(cu, a), source(cu, a), "annotation " + qn + ": annotations with behaviour are outside cir/1");
            }
        }
    }

    private Map<String, Object> foreign(CompilationUnitTree cu, Tree t, String reason) {
        String src = source(cu, t);
        String pos = pos(cu, t);
        refuse(pos, firstLine(src), reason);
        Map<String, Object> st = Json.obj();
        st.put("k", "foreign");
        st.put("lang", "java");
        st.put("src", src);
        st.put("reason", reason);
        st.put("pos", pos);
        return st;
    }
}

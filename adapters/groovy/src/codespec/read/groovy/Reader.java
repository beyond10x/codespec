package codespec.read.groovy;

import codespec.jvm.JvmTypes;
import groovy.lang.GroovyClassLoader;
import java.io.IOException;
import java.lang.reflect.Modifier;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.Deque;
import java.util.Enumeration;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.function.Predicate;
import java.util.stream.Stream;
import org.codehaus.groovy.ast.ASTNode;
import org.codehaus.groovy.ast.AnnotatedNode;
import org.codehaus.groovy.ast.AnnotationNode;
import org.codehaus.groovy.ast.ClassHelper;
import org.codehaus.groovy.ast.ClassNode;
import org.codehaus.groovy.ast.CodeVisitorSupport;
import org.codehaus.groovy.ast.ConstructorNode;
import org.codehaus.groovy.ast.DynamicVariable;
import org.codehaus.groovy.ast.FieldNode;
import org.codehaus.groovy.ast.GenericsType;
import org.codehaus.groovy.ast.MethodNode;
import org.codehaus.groovy.ast.ModuleNode;
import org.codehaus.groovy.ast.Parameter;
import org.codehaus.groovy.ast.PropertyNode;
import org.codehaus.groovy.ast.Variable;
import org.codehaus.groovy.ast.expr.ArgumentListExpression;
import org.codehaus.groovy.ast.expr.ArrayExpression;
import org.codehaus.groovy.ast.expr.AttributeExpression;
import org.codehaus.groovy.ast.expr.BinaryExpression;
import org.codehaus.groovy.ast.expr.BitwiseNegationExpression;
import org.codehaus.groovy.ast.expr.BooleanExpression;
import org.codehaus.groovy.ast.expr.CastExpression;
import org.codehaus.groovy.ast.expr.ClassExpression;
import org.codehaus.groovy.ast.expr.ClosureExpression;
import org.codehaus.groovy.ast.expr.ClosureListExpression;
import org.codehaus.groovy.ast.expr.ConstantExpression;
import org.codehaus.groovy.ast.expr.ConstructorCallExpression;
import org.codehaus.groovy.ast.expr.DeclarationExpression;
import org.codehaus.groovy.ast.expr.ElvisOperatorExpression;
import org.codehaus.groovy.ast.expr.EmptyExpression;
import org.codehaus.groovy.ast.expr.Expression;
import org.codehaus.groovy.ast.expr.FieldExpression;
import org.codehaus.groovy.ast.expr.GStringExpression;
import org.codehaus.groovy.ast.expr.ListExpression;
import org.codehaus.groovy.ast.expr.MapExpression;
import org.codehaus.groovy.ast.expr.MethodCallExpression;
import org.codehaus.groovy.ast.expr.MethodPointerExpression;
import org.codehaus.groovy.ast.expr.NotExpression;
import org.codehaus.groovy.ast.expr.PostfixExpression;
import org.codehaus.groovy.ast.expr.PrefixExpression;
import org.codehaus.groovy.ast.expr.PropertyExpression;
import org.codehaus.groovy.ast.expr.RangeExpression;
import org.codehaus.groovy.ast.expr.SpreadExpression;
import org.codehaus.groovy.ast.expr.SpreadMapExpression;
import org.codehaus.groovy.ast.expr.StaticMethodCallExpression;
import org.codehaus.groovy.ast.expr.TernaryExpression;
import org.codehaus.groovy.ast.expr.TupleExpression;
import org.codehaus.groovy.ast.expr.UnaryMinusExpression;
import org.codehaus.groovy.ast.expr.UnaryPlusExpression;
import org.codehaus.groovy.ast.expr.VariableExpression;
import org.codehaus.groovy.ast.stmt.AssertStatement;
import org.codehaus.groovy.ast.stmt.BlockStatement;
import org.codehaus.groovy.ast.stmt.BreakStatement;
import org.codehaus.groovy.ast.stmt.CaseStatement;
import org.codehaus.groovy.ast.stmt.ContinueStatement;
import org.codehaus.groovy.ast.stmt.EmptyStatement;
import org.codehaus.groovy.ast.stmt.ExpressionStatement;
import org.codehaus.groovy.ast.stmt.ForStatement;
import org.codehaus.groovy.ast.stmt.IfStatement;
import org.codehaus.groovy.ast.stmt.ReturnStatement;
import org.codehaus.groovy.ast.stmt.Statement;
import org.codehaus.groovy.ast.stmt.SwitchStatement;
import org.codehaus.groovy.ast.stmt.SynchronizedStatement;
import org.codehaus.groovy.ast.stmt.ThrowStatement;
import org.codehaus.groovy.ast.stmt.TryCatchStatement;
import org.codehaus.groovy.ast.stmt.WhileStatement;
import org.codehaus.groovy.classgen.ReturnAdder;
import org.codehaus.groovy.classgen.asm.sc.StaticTypesTypeChooser;
import org.codehaus.groovy.control.CompilationFailedException;
import org.codehaus.groovy.control.CompilationUnit;
import org.codehaus.groovy.control.CompilerConfiguration;
import org.codehaus.groovy.control.Phases;
import org.codehaus.groovy.control.SourceUnit;
import org.codehaus.groovy.control.messages.ExceptionMessage;
import org.codehaus.groovy.control.messages.SimpleMessage;
import org.codehaus.groovy.control.messages.SyntaxErrorMessage;
import org.codehaus.groovy.syntax.SyntaxException;
import org.codehaus.groovy.syntax.Types;
import org.codehaus.groovy.transform.stc.ExtensionMethodNode;
import org.codehaus.groovy.transform.stc.StaticTypeCheckingVisitor;
import org.codehaus.groovy.transform.stc.StaticTypesMarker;

/**
 * The Groovy frontend: the {@code .groovy} files of one directory to one cir/1 module, with the
 * types the Groovy 2.4 static type checker infers.
 *
 * <p>How it reads: the files are compiled together with {@code CompilationUnit} through the {@code
 * CANONICALIZATION} phase, so every name resolves, and then {@code StaticTypeCheckingVisitor}, the
 * visitor {@code @TypeChecked} runs at {@code INSTRUCTION_SELECTION}, is run over every class
 * whether annotated or not. The local transforms of {@code INSTRUCTION_SELECTION} itself are not
 * run: {@code @CompileStatic}'s would rewrite the tree the reader reads. Every expression is typed
 * from the checker's inferred type ({@code StaticTypesMarker}, through {@code
 * StaticTypesTypeChooser}), else its declared type; a local or parameter has its declared type, a
 * {@code def} local the type the checker infers for its declaration. A type-checking error does not
 * fail the read: the statement holding it becomes {@code foreign}. A file whose imports or classes
 * do not resolve is refused as a whole at the first such line; the others are read.
 *
 * <p>What it maps, as the Java reader ({@code adapters/java}) maps the same construct:
 *
 * <ul>
 *   <li>A class extending {@code Object} becomes a struct of its instance fields and properties,
 *       its instance methods methods with the receiver {@code this} of type {@code *Class}, its
 *       static methods functions, its constructor the function {@code New<Class>} returning {@code
 *       *Class}, and its {@code static final} constants consts. A value of a read class is {@code
 *       ptr(named)}.
 *   <li>An enum without members of its own becomes an enum over {@code string} whose variants hold
 *       their own names.
 *   <li>Types by the JVM type table every JVM reader shares, {@code codespec.jvm.JvmTypes}. A
 *       {@code def} parameter is {@code any}: every use of it the checker cannot type is refused.
 *   <li>{@code for (x in list)} and {@code for (T x : list)} over a {@code List}; a loop over a
 *       {@code HashMap}'s keys is a {@code for_range} over the map; over a {@code TreeMap}'s keys,
 *       built by {@code new TreeMap<>()} with String or integer keys, it ranges over {@code
 *       sorted_keys}. {@code while}, the three-clause {@code for}, {@code break}, {@code continue},
 *       {@code switch} over an integer, a String or an enum without fall-through.
 *   <li>String concatenation; a GString whose values are all Strings or integers, as {@code
 *       sprintf} with {@code %s} and {@code %d}.
 *   <li>{@code add}, {@code set} and {@code put} only on a collection no other name shares, as the
 *       Java reader reads them; {@code size}, {@code isEmpty}, {@code get}, {@code getOrDefault} with
 *       the zero value, {@code String.equals}, {@code String.isEmpty}, {@code String.format} with
 *       {@code %s} and {@code %d}.
 *   <li>A non-void method's last statement is returned as Groovy returns it ({@code ReturnAdder}).
 * </ul>
 *
 * <p>Everything else is kept as a {@code foreign} statement holding its Groovy source, or, for a
 * declaration cir/1 has no foreign form for, refused at its line. Every reason begins with one
 * category of the closed list in {@code README.md}: {@code dynamic}, {@code closure}, {@code
 * groovy-semantics}, {@code type}, {@code unresolved} and {@code unsupported} here.
 */
public final class Reader {
    /** Why a directory could not be read at all. */
    public static final class Failure extends Exception {
        private static final long serialVersionUID = 1L;

        Failure(String message) {
            super(message);
        }
    }

    static final String DYNAMIC = "dynamic";
    static final String CLOSURE = "closure";
    static final String GROOVY = "groovy-semantics";
    static final String TYPE = "type";
    static final String UNRESOLVED = "unresolved";
    static final String UNSUPPORTED = "unsupported";

    /** A construct outside cir/1; the enclosing statement becomes foreign with this reason. */
    static final class Unsupported extends RuntimeException {
        private static final long serialVersionUID = 1L;

        Unsupported(String category, String detail) {
            super(category + ": " + detail, null, false, false);
        }
    }

    private static Unsupported refuse(String category, String detail) {
        return new Unsupported(category, detail);
    }

    private static Unsupported unsupported(String detail) {
        return new Unsupported(UNSUPPORTED, detail);
    }

    private static final int ACC_ENUM = 0x4000;

    private static final Set<String> HARMLESS_ANNOTATIONS = Set.of(
        "java.lang.Override", "java.lang.Deprecated", "java.lang.SuppressWarnings", "groovy.transform.CompileStatic",
        "groovy.transform.TypeChecked");

    /** Methods through which a Groovy class answers names it does not declare. */
    private static final Set<String> DYNAMIC_HOOKS = Set.of(
        "methodMissing", "propertyMissing", "$static_methodMissing", "$static_propertyMissing", "invokeMethod",
        "getProperty", "setProperty");

    private static final String STC_PREFIX = "[Static type checking] - ";

    /** One source file that is read. */
    private static final class Unit {
        final SourceUnit su;
        final String file;
        final String[] lines;

        Unit(SourceUnit su, String file, String text) {
            this.su = su;
            this.file = file;
            this.lines = text.split("\n", -1);
        }
    }

    /** A type-checking error at a position of a read file. */
    private static final class Err {
        final Unit unit;
        final int line;
        final int col;
        final String msg;
        boolean consumed;

        Err(Unit unit, int line, int col, String msg) {
            this.unit = unit;
            this.line = line;
            this.col = col;
            this.msg = msg;
        }
    }

    /** A function or method the reader reads: its cir/1 signature and where it came from. */
    private static final class Fn {
        String name;
        boolean exported;
        boolean isStatic;
        boolean isCtor;
        ClassNode cls;
        List<String> paramNames = new ArrayList<>();
        List<Parameter> paramNodes = new ArrayList<>();
        List<Map<String, Object>> params = new ArrayList<>();
        /** The result, or null for none. */
        Map<String, Object> result;
        /** The method, or null for a synthesized constructor. */
        MethodNode node;

        String key() {
            return isStatic || isCtor ? name : cls.getNameWithoutPackage() + "." + name;
        }

        Map<String, Object> resultType() {
            return result == null ? tUnit() : result;
        }
    }

    /** The state of one function body being read. */
    private final class Ctx {
        final Fn fn;
        final Unit unit;
        final boolean hasThis;
        final int fallbackLine;
        final Set<String> used = new HashSet<>();
        final Deque<String> breaks = new ArrayDeque<>();
        /** The collections the enclosing for-each loops iterate, by the variable or field naming them. */
        final Deque<Object> iterating = new ArrayDeque<>();
        /** The cir/1 type of each local and parameter, by the node Groovy resolves a name to. */
        final Map<Object, Map<String, Object>> vars = new IdentityHashMap<>();
        /** {@code def} locals whose type changes after their declaration; every use is refused. */
        final Set<Object> widened = Collections.newSetFromMap(new IdentityHashMap<>());
        /** How many for-each loops enclose the statement being read. */
        int forEach;

        Ctx(Fn fn, Unit unit, boolean hasThis, int fallbackLine) {
            this.fn = fn;
            this.unit = unit;
            this.hasThis = hasThis;
            this.fallbackLine = fallbackLine;
        }
    }

    private final StaticTypesTypeChooser chooser = new StaticTypesTypeChooser();
    private final List<Unit> units = new ArrayList<>();
    private final Map<ClassNode, Unit> unitOf = new IdentityHashMap<>();
    private final List<Err> errs = new ArrayList<>();
    /** Top-level classes of the read files, in file then source order. */
    private final List<ClassNode> decls = new ArrayList<>();
    private final Map<String, ClassNode> structs = new LinkedHashMap<>();
    private final Map<String, ClassNode> enums = new LinkedHashMap<>();
    private final Map<FieldNode, String> consts = new IdentityHashMap<>();
    private final Map<MethodNode, Fn> fns = new IdentityHashMap<>();
    private final Map<ClassNode, Fn> ctors = new IdentityHashMap<>();

    private final List<Object> outTypes = new ArrayList<>();
    private final List<Object> outConsts = new ArrayList<>();
    private final List<Object> outFuncs = new ArrayList<>();
    private final List<Object> layout = new ArrayList<>();
    private final List<Map<String, Object>> refusals = new ArrayList<>();

    private Reader() {}

    // ---- compiling ----

    /** Read every {@code .groovy} file directly in {@code dir}, all of one package. */
    public static Map<String, Object> read(Path dir) throws Failure {
        List<Path> files;
        try (Stream<Path> s = Files.list(dir)) {
            files = s.filter(p -> p.getFileName().toString().endsWith(".groovy") && Files.isRegularFile(p))
                .sorted(Comparator.comparing(p -> p.getFileName().toString()))
                .toList();
        } catch (IOException e) {
            throw new Failure(dir + ": " + e.getMessage());
        }
        if (files.isEmpty()) {
            throw new Failure(dir + ": no .groovy files");
        }
        return read(files, List.of());
    }

    /**
     * Read {@code files}, all of one package, compiled together with {@code resolveOnly}: those are
     * compiled so that references to them resolve, and never read.
     */
    public static Map<String, Object> read(List<Path> files, List<Path> resolveOnly) throws Failure {
        Reader r = new Reader();
        List<Path> remaining = new ArrayList<>();
        for (Path f : files) {
            remaining.add(f.toAbsolutePath().normalize());
        }
        List<Path> resolve = new ArrayList<>();
        for (Path f : resolveOnly) {
            resolve.add(f.toAbsolutePath().normalize());
        }
        Map<String, String> packages = new HashMap<>();
        while (true) {
            CompilationUnit cu = newUnit();
            for (Path f : remaining) {
                cu.addSource(f.toFile());
            }
            for (Path f : resolve) {
                cu.addSource(f.toFile());
            }
            try {
                cu.compile(Phases.CANONICALIZATION);
                r.typeCheck(cu, remaining);
                break;
            } catch (CompilationFailedException e) {
                packagesOf(cu, packages);
                Set<Path> refused = r.unresolvedFiles(cu, remaining, resolve);
                remaining.removeAll(refused);
                if (remaining.isEmpty()) {
                    return r.module(packageOf(files, packages));
                }
            }
        }
        return r.module(r.packageName());
    }

    private static CompilationUnit newUnit() {
        CompilerConfiguration config = new CompilerConfiguration();
        config.setSourceEncoding("UTF-8");
        GroovyClassLoader loader = new GroovyClassLoader(new Hidden(Reader.class.getClassLoader()), config);
        return new CompilationUnit(config, null, loader);
    }

    /** The reader's own classes are not on the compile classpath of the code it reads. */
    private static final class Hidden extends ClassLoader {
        Hidden(ClassLoader parent) {
            super(parent);
        }

        @Override
        protected Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
            if (name.startsWith("codespec.")) {
                throw new ClassNotFoundException(name);
            }
            return super.loadClass(name, resolve);
        }

        @Override
        public URL getResource(String name) {
            return name.startsWith("codespec/") ? null : super.getResource(name);
        }

        @Override
        public Enumeration<URL> getResources(String name) throws IOException {
            return name.startsWith("codespec/") ? Collections.emptyEnumeration() : super.getResources(name);
        }
    }

    private static void packagesOf(CompilationUnit cu, Map<String, String> packages) {
        Iterator<SourceUnit> it = cu.iterator();
        while (it.hasNext()) {
            SourceUnit su = it.next();
            if (su.getAST() != null) {
                packages.put(su.getName(), packageOf(su.getAST()));
            }
        }
    }

    private static String packageOf(ModuleNode m) {
        String p = m.getPackageName();
        if (p == null) {
            return "";
        }
        return p.endsWith(".") ? p.substring(0, p.length() - 1) : p;
    }

    private static String packageOf(List<Path> files, Map<String, String> packages) throws Failure {
        for (Path f : files) {
            String p = packages.get(f.toAbsolutePath().normalize().toString());
            if (p != null && !p.isEmpty()) {
                return p;
            }
        }
        throw new Failure("no file declares a package");
    }

    /**
     * The read files a failed compilation refuses: each whose only errors are classes that do not
     * resolve, refused at the first such line. Any other error fails the read.
     */
    private Set<Path> unresolvedFiles(CompilationUnit cu, List<Path> remaining, List<Path> resolve) throws Failure {
        Map<String, SyntaxException> first = new LinkedHashMap<>();
        List<String> other = new ArrayList<>();
        List<?> messages = cu.getErrorCollector().getErrors();
        for (Object m : messages == null ? List.of() : messages) {
            if (m instanceof SyntaxErrorMessage) {
                SyntaxException se = ((SyntaxErrorMessage) m).getCause();
                String text = se.getOriginalMessage().trim();
                String where = Path.of(String.valueOf(se.getSourceLocator())).getFileName() + ":" + se.getStartLine() + ": ";
                if (text.startsWith("unable to resolve class ")) {
                    SyntaxException seen = first.get(se.getSourceLocator());
                    if (seen == null || se.getStartLine() < seen.getStartLine()) {
                        first.put(se.getSourceLocator(), se);
                    }
                } else {
                    other.add(where + text);
                }
            } else if (m instanceof ExceptionMessage) {
                other.add(String.valueOf(((ExceptionMessage) m).getCause()));
            } else if (m instanceof SimpleMessage) {
                other.add(((SimpleMessage) m).getMessage());
            } else {
                other.add(String.valueOf(m));
            }
        }
        if (!other.isEmpty() || first.isEmpty()) {
            Collections.sort(other);
            throw new Failure(other.isEmpty() ? "compilation failed" : String.join("\n", other));
        }
        Set<Path> refused = new HashSet<>();
        for (Map.Entry<String, SyntaxException> e : first.entrySet()) {
            Path p = Path.of(e.getKey()).toAbsolutePath().normalize();
            SyntaxException se = e.getValue();
            String cls = se.getOriginalMessage().trim().substring("unable to resolve class ".length()).trim();
            int sp = cls.indexOf(' ');
            if (sp >= 0) {
                cls = cls.substring(0, sp);
            }
            if (!remaining.contains(p)) {
                throw new Failure(p.getFileName() + ":" + se.getStartLine() + ": unable to resolve class " + cls
                    + " in a file that is compiled but not read");
            }
            String line = lineText(p, se.getStartLine());
            addRefusal(p.getFileName() + ":" + se.getStartLine(), line, UNRESOLVED + ": " + cls);
            refused.add(p);
        }
        return refused;
    }

    private static String lineText(Path p, int line) {
        try {
            List<String> lines = Files.readAllLines(p, StandardCharsets.UTF_8);
            return line >= 1 && line <= lines.size() ? lines.get(line - 1).strip() : p.getFileName().toString();
        } catch (IOException e) {
            return p.getFileName().toString();
        }
    }

    /** Run the static type checker over every class of every read file and keep its errors. */
    private void typeCheck(CompilationUnit cu, List<Path> read) throws Failure {
        Set<String> names = new HashSet<>();
        for (Path p : read) {
            names.add(p.toString());
        }
        List<SourceUnit> sus = new ArrayList<>();
        Iterator<SourceUnit> it = cu.iterator();
        while (it.hasNext()) {
            SourceUnit su = it.next();
            if (names.contains(Path.of(su.getName()).toAbsolutePath().normalize().toString())) {
                sus.add(su);
            }
        }
        sus.sort(Comparator.comparing(su -> Path.of(su.getName()).getFileName().toString()));
        Map<String, Unit> byName = new HashMap<>();
        for (SourceUnit su : sus) {
            String text;
            try {
                text = Files.readString(Path.of(su.getName()), StandardCharsets.UTF_8);
            } catch (IOException e) {
                throw new Failure(su.getName() + ": " + e.getMessage());
            }
            Unit u = new Unit(su, Path.of(su.getName()).getFileName().toString(), text);
            units.add(u);
            byName.put(su.getName(), u);
            List<ClassNode> classes = new ArrayList<>(su.getAST().getClasses());
            classes.sort(Comparator.comparingInt(ClassNode::getLineNumber));
            for (ClassNode cn : classes) {
                unitOf.put(cn, u);
                decls.add(cn);
            }
        }
        int before = errorCount(cu);
        for (ClassNode cn : decls) {
            if (cn.isInterface() || cn.isAnnotationDefinition()) {
                continue;
            }
            StaticTypeCheckingVisitor v = new StaticTypeCheckingVisitor(unitOf.get(cn).su, cn);
            v.setCompilationUnit(cu);
            v.initialize();
            try {
                v.visitClass(cn);
                v.performSecondPass();
            } catch (RuntimeException e) {
                throw new Failure(unitOf.get(cn).file + ": the Groovy type checker failed on class " + cn.getName() + ": " + e);
            }
        }
        List<?> messages = cu.getErrorCollector().getErrors();
        if (messages == null) {
            return;
        }
        for (Object m : messages.subList(before, messages.size())) {
            if (!(m instanceof SyntaxErrorMessage)) {
                throw new Failure("the Groovy type checker reported " + m);
            }
            SyntaxException se = ((SyntaxErrorMessage) m).getCause();
            Unit u = byName.get(se.getSourceLocator());
            if (u == null) {
                throw new Failure("a type-checking error outside the read files: " + se.getOriginalMessage());
            }
            String msg = se.getOriginalMessage().trim();
            if (msg.startsWith(STC_PREFIX)) {
                msg = msg.substring(STC_PREFIX.length());
            }
            errs.add(new Err(u, se.getStartLine(), se.getStartColumn(), msg.replaceAll("\\s+", " ")));
        }
    }

    private static int errorCount(CompilationUnit cu) {
        List<?> m = cu.getErrorCollector().getErrors();
        return m == null ? 0 : m.size();
    }

    // ---- the module ----

    private String packageName() throws Failure {
        String pkg = null;
        for (Unit u : units) {
            String p = packageOf(u.su.getAST());
            if (p.isEmpty()) {
                throw new Failure(u.file + ": no package declaration");
            }
            if (pkg != null && !pkg.equals(p)) {
                throw new Failure(u.file + ": package " + p + ", other files are in " + pkg);
            }
            pkg = p;
        }
        return pkg;
    }

    private Map<String, Object> module(String pkg) {
        classify();
        signatures();
        for (ClassNode cn : decls) {
            emit(cn);
        }
        for (Err e : errs) {
            if (!e.consumed) {
                addRefusal(e.unit.file + ":" + e.line, "type check", DYNAMIC + ": " + e.msg);
            }
        }
        refusals.sort(Comparator.comparing((Map<String, Object> r) -> file((String) r.get("pos")))
            .thenComparingInt(r -> line((String) r.get("pos"))));

        Map<String, Object> doc = Json.obj();
        doc.put("format", "cir/1");
        doc.put("name", pkg.substring(pkg.lastIndexOf('.') + 1));
        Map<String, Object> reader = Json.obj();
        reader.put("language", "groovy");
        reader.put("name", "codespec-read-groovy");
        reader.put("version", Main.VERSION);
        doc.put("reader", reader);
        doc.put("types", outTypes);
        if (!outConsts.isEmpty()) {
            doc.put("consts", outConsts);
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

    // ---- classification of the top-level classes ----

    private static boolean userNode(ASTNode n) {
        return n.getLineNumber() > 0;
    }

    /** The user-declared instance and static fields of a class, properties included, in source order. */
    private static List<FieldNode> userFields(ClassNode cn) {
        List<FieldNode> out = new ArrayList<>();
        for (FieldNode f : cn.getFields()) {
            // The field behind a property is marked synthetic; it is still the user's.
            boolean declared = !f.isSynthetic() || cn.getProperty(f.getName()) != null;
            if (userNode(f) && declared && !f.getName().startsWith("$") && !f.getName().startsWith("__")) {
                out.add(f);
            }
        }
        out.sort(Comparator.comparingInt(FieldNode::getLineNumber).thenComparingInt(FieldNode::getColumnNumber));
        return out;
    }

    /** The user-declared methods and constructors of a class, in source order. */
    private static List<MethodNode> userMethods(ClassNode cn) {
        List<MethodNode> out = new ArrayList<>();
        for (MethodNode m : cn.getDeclaredConstructors()) {
            if (userNode(m)) {
                out.add(m);
            }
        }
        for (MethodNode m : cn.getMethods()) {
            if (userNode(m) && !m.isSynthetic() && !m.getName().startsWith("<")) {
                out.add(m);
            }
        }
        out.sort(Comparator.comparingInt(MethodNode::getLineNumber).thenComparingInt(MethodNode::getColumnNumber));
        return out;
    }

    private void classify() {
        for (ClassNode cn : decls) {
            Unit u = unitOf.get(cn);
            String name = cn.getNameWithoutPackage();
            String why = null;
            if (cn.isScript()) {
                why = "statements outside a class: a script is outside cir/1";
            } else if (cn.getOuterClass() != null) {
                why = "nested class: only top-level classes are read";
            } else {
                why = annotations(u, cn);
                if (why == null) {
                    why = classShape(cn);
                }
            }
            if (why != null) {
                refuseDecl(u, cn, name, why);
                consume(u, cn);
                continue;
            }
            if (cn.isEnum()) {
                enums.put(cn.getName(), cn);
            } else {
                structs.put(cn.getName(), cn);
            }
        }
        // A struct a field of which cannot be read is refused at each such field; that can make the
        // field types of others unreadable in turn.
        boolean changed = true;
        while (changed) {
            changed = false;
            for (ClassNode cn : decls) {
                if (!structs.containsKey(cn.getName())) {
                    continue;
                }
                Map<FieldNode, String> bad = badFields(cn);
                if (bad.isEmpty()) {
                    continue;
                }
                structs.remove(cn.getName());
                Unit u = unitOf.get(cn);
                for (Map.Entry<FieldNode, String> e : bad.entrySet()) {
                    refuseDecl(u, e.getKey(), cn.getNameWithoutPackage() + "." + e.getKey().getName(), e.getValue());
                }
                consume(u, cn);
                changed = true;
            }
        }
    }

    /** Why a class is not read as a struct or an enum, or null. */
    private String classShape(ClassNode cn) {
        if (cn.isAnnotationDefinition()) {
            return UNSUPPORTED + ": annotation type: annotations with behaviour are outside cir/1";
        }
        if (cn.isInterface()) {
            return UNSUPPORTED + ": interface or trait: dispatch through an interface is outside the reader's subset";
        }
        if (cn.getGenericsTypes() != null && cn.getGenericsTypes().length > 0) {
            return UNSUPPORTED + ": generic class: generics declared by the code are outside cir/1";
        }
        if (Modifier.isAbstract(cn.getModifiers())) {
            return UNSUPPORTED + ": abstract class: inheritance is outside cir/1";
        }
        if (cn.isEnum()) {
            return enumShape(cn);
        }
        if (!cn.getSuperClass().getName().equals("java.lang.Object")) {
            return UNSUPPORTED + ": class extends " + cn.getSuperClass().getName() + ": inheritance is outside cir/1";
        }
        for (ClassNode i : cn.getInterfaces()) {
            if (!i.getName().equals("groovy.lang.GroovyObject")) {
                return UNSUPPORTED + ": implements " + i.getName() + ": interfaces are outside the reader's subset";
            }
        }
        for (String hook : List.of("getProperty", "setProperty")) {
            for (MethodNode m : cn.getMethods(hook)) {
                if (userNode(m) && !m.isStatic()) {
                    return DYNAMIC + ": " + hook + " declared: Groovy sends every property access of the class through it";
                }
            }
        }
        if (!cn.getObjectInitializerStatements().isEmpty()) {
            return UNSUPPORTED + ": initializer block: cir/1 has no constructor phase for it";
        }
        return null;
    }

    private String enumShape(ClassNode cn) {
        if (cn.getInterfaces().length > 0) {
            for (ClassNode i : cn.getInterfaces()) {
                if (!i.getName().equals("groovy.lang.GroovyObject")) {
                    return UNSUPPORTED + ": enum implementing an interface";
                }
            }
        }
        for (FieldNode f : userFields(cn)) {
            if ((f.getModifiers() & ACC_ENUM) == 0) {
                return UNSUPPORTED + ": enum with fields, constructors or methods of its own";
            }
            Expression init = f.getInitialExpression();
            if (init instanceof ListExpression && !((ListExpression) init).getExpressions().isEmpty()) {
                return UNSUPPORTED + ": enum constant with arguments or a body";
            }
        }
        if (!userMethods(cn).isEmpty()) {
            return UNSUPPORTED + ": enum with fields, constructors or methods of its own";
        }
        return null;
    }

    /**
     * The instance fields of a struct that are not read, with the reason: a type outside the table,
     * {@code def}, or a reference type with no initializer that some constructor does not assign
     * first, which holds {@code null} until assigned.
     */
    private Map<FieldNode, String> badFields(ClassNode cn) {
        Map<FieldNode, String> out = new LinkedHashMap<>();
        for (FieldNode f : userFields(cn)) {
            if (f.isStatic()) {
                continue;
            }
            String kind = cn.getProperty(f.getName()) != null ? "property" : "field";
            if (f.isDynamicTyped()) {
                out.put(f, DYNAMIC + ": " + kind + " " + f.getName() + " is declared def: its type is whatever it is given");
                continue;
            }
            try {
                typeOf(f.getType());
            } catch (Unsupported u) {
                out.put(f, u.getMessage());
                continue;
            }
            if (!ClassHelper.isPrimitiveType(f.getType()) && !f.hasInitialExpression() && !assignedInEveryConstructor(cn, f)) {
                out.put(f, TYPE + ": " + kind + " " + f.getName() + " holds null until assigned, which no cir/1 zero value is:"
                    + " give it an initializer or assign it first in every constructor");
            }
        }
        return out;
    }

    /**
     * Whether every constructor of the class, there being one, starts with a run of {@code this.f =
     * <value>} or {@code f = <value>} statements, reading nothing of {@code this}, that assigns {@code
     * field}.
     */
    private boolean assignedInEveryConstructor(ClassNode cn, FieldNode field) {
        List<ConstructorNode> cs = new ArrayList<>();
        for (ConstructorNode c : cn.getDeclaredConstructors()) {
            if (userNode(c)) {
                cs.add(c);
            }
        }
        if (cs.isEmpty()) {
            return false;
        }
        for (ConstructorNode c : cs) {
            if (!assignedFirst(c, field)) {
                return false;
            }
        }
        return true;
    }

    private boolean assignedFirst(ConstructorNode c, FieldNode field) {
        if (!(c.getCode() instanceof BlockStatement)) {
            return false;
        }
        for (Statement s : ((BlockStatement) c.getCode()).getStatements()) {
            if (!(s instanceof ExpressionStatement) || !(((ExpressionStatement) s).getExpression() instanceof BinaryExpression)) {
                return false;
            }
            BinaryExpression b = (BinaryExpression) ((ExpressionStatement) s).getExpression();
            if (b.getOperation().getType() != Types.EQUAL || readsThis(b.getRightExpression())) {
                return false;
            }
            FieldNode target = ownField(b.getLeftExpression(), c.getDeclaringClass());
            if (target == null) {
                return false;
            }
            if (target == field) {
                return true;
            }
        }
        return false;
    }

    /** The instance field of {@code cls} an expression names as {@code f} or {@code this.f}, or null. */
    private static FieldNode ownField(Expression e, ClassNode cls) {
        if (e instanceof VariableExpression) {
            Variable v = ((VariableExpression) e).getAccessedVariable();
            FieldNode f = v instanceof PropertyNode ? ((PropertyNode) v).getField() : v instanceof FieldNode ? (FieldNode) v : null;
            return f != null && !f.isStatic() && f.getOwner().getName().equals(cls.getName()) ? f : null;
        }
        if (e instanceof PropertyExpression && !(e instanceof AttributeExpression) && isThis(((PropertyExpression) e).getObjectExpression())) {
            FieldNode f = cls.getField(((PropertyExpression) e).getPropertyAsString());
            return f != null && !f.isStatic() ? f : null;
        }
        return null;
    }

    private static boolean isThis(Expression e) {
        return e instanceof VariableExpression && ((VariableExpression) e).isThisExpression();
    }

    /** Whether an expression reads {@code this}: by name, through a field or through an instance method. */
    private static boolean readsThis(Expression e) {
        boolean[] found = {false};
        e.visit(new CodeVisitorSupport() {
            @Override
            public void visitVariableExpression(VariableExpression v) {
                Variable a = v.getAccessedVariable();
                if (v.isThisExpression() || v.isSuperExpression() || a instanceof DynamicVariable
                    || a instanceof FieldNode && !((FieldNode) a).isStatic()
                    || a instanceof PropertyNode && !((PropertyNode) a).isStatic()) {
                    found[0] = true;
                }
            }

            @Override
            public void visitMethodCallExpression(MethodCallExpression m) {
                if (m.isImplicitThis()) {
                    found[0] = true;
                }
                super.visitMethodCallExpression(m);
            }

            @Override
            public void visitClosureExpression(ClosureExpression c) {
                found[0] = true;
            }
        });
        return found[0];
    }

    /**
     * Refuses the annotations with behaviour on a node at their lines. Returns the reason the node
     * itself is not read, when an annotation turns type checking off for it.
     */
    private String annotations(Unit u, AnnotatedNode n) {
        String why = null;
        for (AnnotationNode a : n.getAnnotations()) {
            String qn = a.getClassNode().getName();
            if (qn.equals("groovy.transform.CompileDynamic") || HARMLESS_ANNOTATIONS.contains(qn) && skips(a)) {
                why = DYNAMIC + ": " + "@" + a.getClassNode().getNameWithoutPackage()
                    + " turns type checking off, so nothing here has an inferred type";
            } else if (!HARMLESS_ANNOTATIONS.contains(qn)) {
                addRefusal(pos(u, a, n.getLineNumber()), "@" + a.getClassNode().getNameWithoutPackage(),
                    UNSUPPORTED + ": annotation " + qn + ": annotations with behaviour are outside cir/1");
            }
        }
        return why;
    }

    /** Whether a {@code @TypeChecked} or {@code @CompileStatic} annotation says {@code SKIP}. */
    private static boolean skips(AnnotationNode a) {
        Expression v = a.getMember("value");
        return v != null && v.getText().endsWith("SKIP");
    }

    // ---- signatures ----

    private void signatures() {
        Set<String> funcNames = new HashSet<>();
        for (ClassNode cn : structs.values()) {
            funcNames.add("New" + cn.getNameWithoutPackage());
        }
        Set<String> constNames = new HashSet<>();
        for (ClassNode cn : decls) {
            if (!structs.containsKey(cn.getName())) {
                continue;
            }
            Unit u = unitOf.get(cn);
            String cname = cn.getNameWithoutPackage();
            for (FieldNode f : userFields(cn)) {
                if (!f.isStatic()) {
                    continue;
                }
                String why = null;
                if (!f.isFinal() || constValue(f) == null) {
                    why = UNSUPPORTED + ": static field other than a constant: module-level state is outside cir/1";
                } else if (funcNames.contains(f.getName()) || constNames.contains(f.getName())) {
                    why = UNSUPPORTED + ": a second constant named " + f.getName();
                } else {
                    try {
                        constLit(constValue(f), typeOf(f.getType()));
                    } catch (Unsupported e) {
                        why = e.getMessage();
                    }
                }
                if (why != null) {
                    refuseDecl(u, f, cname + "." + f.getName(), why);
                    consume(u, f);
                } else {
                    consts.put(f, f.getName());
                    constNames.add(f.getName());
                }
            }
            Set<String> methodNames = new HashSet<>();
            boolean explicitCtor = false;
            for (MethodNode m : userMethods(cn)) {
                boolean ctor = m instanceof ConstructorNode;
                String name = ctor ? "New" + cname : m.getName();
                String what = ctor ? cname + "(…)" : cname + "." + name;
                String why = annotations(u, m);
                for (Parameter p : m.getParameters()) {
                    String pw = annotations(u, p);
                    why = why == null ? pw : why;
                }
                if (why != null) {
                    // turned off by an annotation
                } else if (!ctor && DYNAMIC_HOOKS.contains(name)) {
                    why = DYNAMIC + ": " + name + " declared: Groovy calls it at run time for names the class does not declare";
                } else if (ctor && explicitCtor) {
                    why = UNSUPPORTED + ": a second constructor: overloading is outside cir/1";
                } else if (!ctor && methodNames.contains(name)) {
                    why = UNSUPPORTED + ": a second method named " + name + ": overloading is outside cir/1";
                } else if (!ctor && m.isStatic() && funcNames.contains(name)) {
                    why = UNSUPPORTED + ": a second function named " + name + " in the package";
                } else if (m.getGenericsTypes() != null && m.getGenericsTypes().length > 0) {
                    why = UNSUPPORTED + ": generic method: generics declared by the code are outside cir/1";
                } else if (m.isAbstract() || m.getCode() == null) {
                    why = UNSUPPORTED + ": method without a body";
                } else {
                    for (Parameter p : m.getParameters()) {
                        if (p.hasInitialExpression()) {
                            why = GROOVY + ": parameter " + p.getName() + " has a default value: Groovy adds an overload"
                                + " without it";
                            break;
                        }
                    }
                    if (why == null && !ctor && m.isDynamicReturnType()) {
                        why = DYNAMIC + ": " + name + " returns def: its result has the type of whatever it returns";
                    }
                }
                Fn fn = null;
                if (why == null) {
                    try {
                        fn = signature(cn, m, name, ctor);
                    } catch (Unsupported e) {
                        why = e.getMessage();
                    }
                }
                if (why != null) {
                    refuseDecl(u, m, what, why);
                    consume(u, m);
                    continue;
                }
                fns.put(m, fn);
                if (ctor) {
                    explicitCtor = true;
                    ctors.put(cn, fn);
                } else {
                    methodNames.add(name);
                    if (fn.isStatic) {
                        funcNames.add(name);
                    }
                }
            }
            if (!explicitCtor) {
                Fn fn = new Fn();
                fn.name = "New" + cname;
                fn.exported = Modifier.isPublic(cn.getModifiers());
                fn.isCtor = true;
                fn.cls = cn;
                fn.result = tPtr(tNamed(cname));
                ctors.put(cn, fn);
            }
        }
    }

    private Fn signature(ClassNode cn, MethodNode m, String name, boolean ctor) {
        Fn fn = new Fn();
        fn.name = name;
        fn.exported = m.isPublic();
        fn.isStatic = m.isStatic();
        fn.isCtor = ctor;
        fn.cls = cn;
        fn.node = m;
        for (Parameter p : m.getParameters()) {
            fn.paramNames.add(p.getName());
            fn.paramNodes.add(p);
            fn.params.add(p.isDynamicTyped() ? kind("any") : typeOf(p.getOriginType()));
        }
        if (ctor) {
            fn.result = tPtr(tNamed(cn.getNameWithoutPackage()));
        } else if (!m.isVoidMethod()) {
            fn.result = typeOf(m.getReturnType());
        }
        return fn;
    }

    /** A static final field's constant value, or null when its initializer is not a literal. */
    private static Object constValue(FieldNode f) {
        Expression e = f.getInitialExpression();
        if (e instanceof ConstantExpression) {
            return ((ConstantExpression) e).getValue();
        }
        if (e instanceof UnaryMinusExpression && ((UnaryMinusExpression) e).getExpression() instanceof ConstantExpression) {
            Object v = ((ConstantExpression) ((UnaryMinusExpression) e).getExpression()).getValue();
            if (v instanceof Integer) {
                return -(Integer) v;
            }
            if (v instanceof Long) {
                return -(Long) v;
            }
        }
        return null;
    }

    // ---- emitting declarations ----

    private void emit(ClassNode cn) {
        Unit u = unitOf.get(cn);
        String name = cn.getNameWithoutPackage();
        if (enums.containsKey(cn.getName())) {
            Map<String, Object> d = Json.obj();
            d.put("k", "enum");
            d.put("name", name);
            d.put("exported", Modifier.isPublic(cn.getModifiers()));
            d.put("under", tString());
            List<Object> variants = new ArrayList<>();
            for (FieldNode f : userFields(cn)) {
                Map<String, Object> v = Json.obj();
                v.put("name", f.getName());
                v.put("value", lit(quote(f.getName()), tString()));
                variants.add(v);
            }
            d.put("variants", variants);
            d.put("pos", declPos(u, cn));
            outTypes.add(d);
            layout.add("type:" + name);
            return;
        }
        if (!structs.containsKey(cn.getName())) {
            return;
        }
        Map<String, Object> d = Json.obj();
        d.put("k", "struct");
        d.put("name", name);
        d.put("exported", Modifier.isPublic(cn.getModifiers()));
        List<Object> fields = new ArrayList<>();
        for (FieldNode f : userFields(cn)) {
            if (!f.isStatic()) {
                Map<String, Object> fm = Json.obj();
                fm.put("name", f.getName());
                fm.put("t", typeOf(f.getType()));
                fields.add(fm);
            }
        }
        d.put("fields", fields);
        d.put("pos", declPos(u, cn));
        outTypes.add(d);
        layout.add("type:" + name);
        Fn ctor = ctors.get(cn);
        if (ctor.node == null) {
            emitFunc(ctor);
        }
        List<AnnotatedNode> members = new ArrayList<>();
        for (FieldNode f : userFields(cn)) {
            if (consts.containsKey(f)) {
                members.add(f);
            }
        }
        for (MethodNode m : userMethods(cn)) {
            if (fns.containsKey(m)) {
                members.add(m);
            }
        }
        members.sort(Comparator.comparingInt(AnnotatedNode::getLineNumber).thenComparingInt(AnnotatedNode::getColumnNumber));
        for (AnnotatedNode m : members) {
            if (m instanceof FieldNode) {
                FieldNode f = (FieldNode) m;
                Map<String, Object> c = Json.obj();
                Map<String, Object> t = typeOf(f.getType());
                c.put("name", consts.get(f));
                c.put("t", t);
                c.put("value", constLit(constValue(f), t));
                c.put("pos", declPos(u, f));
                outConsts.add(c);
                layout.add("const:" + consts.get(f));
            } else {
                emitFunc(fns.get((MethodNode) m));
            }
        }
    }

    private void emitFunc(Fn fn) {
        Unit u = unitOf.get(fn.cls);
        int last = fn.node != null ? fn.node.getLastLineNumber() : fn.cls.getLineNumber();
        Ctx ctx = new Ctx(fn, u, !fn.isStatic, last);
        ctx.used.add("this");
        ctx.used.addAll(fn.paramNames);
        for (int i = 0; i < fn.paramNodes.size(); i++) {
            ctx.vars.put(fn.paramNodes.get(i), fn.params.get(i));
        }
        Map<String, Object> f = Json.obj();
        f.put("name", fn.name);
        f.put("exported", fn.exported);
        if (!fn.isStatic && !fn.isCtor) {
            Map<String, Object> recv = Json.obj();
            recv.put("name", "this");
            recv.put("t", tPtr(tNamed(fn.cls.getNameWithoutPackage())));
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
        f.put("results", fn.result == null ? new ArrayList<Object>() : new ArrayList<Object>(List.of(fn.result)));
        List<Object> body = new ArrayList<>();
        String pos;
        if (fn.node != null) {
            MethodNode m = fn.node;
            pos = declPos(u, m);
            if (!fn.isCtor) {
                new ReturnAdder().visitMethod(m);
            }
            m.getCode().visit(new CodeVisitorSupport() {
                @Override
                public void visitDeclarationExpression(DeclarationExpression d) {
                    if (!d.isMultipleAssignmentDeclaration()) {
                        ctx.used.add(d.getVariableExpression().getName());
                    }
                    super.visitDeclarationExpression(d);
                }
            });
            List<Statement> stmts = statements(m.getCode());
            if (fn.isCtor) {
                body.add(thisAlloc(fn, ctx, pos));
            }
            for (Statement s : stmts) {
                body.addAll(stmt(s, ctx));
            }
            if (fn.isCtor && (stmts.isEmpty() || !exits(stmts.get(stmts.size() - 1)))) {
                body.add(ret(successResults(ctx, null), pos));
            }
        } else {
            pos = declPos(u, fn.cls);
            body.add(thisAlloc(fn, ctx, pos));
            body.add(ret(successResults(ctx, null), pos));
        }
        f.put("body", body);
        f.put("pos", pos);
        outFuncs.add(f);
        layout.add("func:" + fn.key());
    }

    private static List<Statement> statements(Statement s) {
        if (s instanceof BlockStatement) {
            return ((BlockStatement) s).getStatements();
        }
        if (s == null || s instanceof EmptyStatement) {
            return List.of();
        }
        return List.of(s);
    }

    /** {@code this := &Class{<field initializers>}}, the first statement of every constructor. */
    private Map<String, Object> thisAlloc(Fn fn, Ctx ctor, String pos) {
        Ctx noThis = new Ctx(fn, ctor.unit, false, ctor.fallbackLine);
        String name = fn.cls.getNameWithoutPackage();
        List<Object> inits = new ArrayList<>();
        for (FieldNode f : userFields(fn.cls)) {
            if (f.isStatic() || !f.hasInitialExpression()) {
                continue;
            }
            Unit u = ctor.unit;
            int mark = refusals.size();
            try {
                Map<String, Object> fi = Json.obj();
                Map<String, Object> t = typeOf(f.getType());
                fi.put("name", f.getName());
                fi.put("value", coerce(value(f.getInitialExpression(), t, noThis), t));
                checkTyped(fi);
                Err e = firstError(u, f);
                if (e != null) {
                    throw refuse(DYNAMIC, e.msg);
                }
                inits.add(fi);
            } catch (Unsupported e) {
                truncate(mark);
                consume(u, f);
                return foreign(u, f, e.getMessage(), ctor);
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
    private static boolean exits(Statement s) {
        if (s instanceof ReturnStatement || s instanceof ThrowStatement) {
            return true;
        }
        if (s instanceof BlockStatement) {
            List<Statement> l = ((BlockStatement) s).getStatements();
            return !l.isEmpty() && exits(l.get(l.size() - 1));
        }
        if (s instanceof IfStatement) {
            IfStatement i = (IfStatement) s;
            return !(i.getElseBlock() instanceof EmptyStatement) && exits(i.getIfBlock()) && exits(i.getElseBlock());
        }
        return false;
    }

    // ---- statements ----

    private List<Object> body(Statement s, Ctx ctx) {
        List<Object> out = new ArrayList<>();
        for (Statement c : statements(s)) {
            out.addAll(stmt(c, ctx));
        }
        return out;
    }

    private List<Object> inside(String target, Statement s, Ctx ctx) {
        ctx.breaks.push(target);
        try {
            return body(s, ctx);
        } finally {
            ctx.breaks.pop();
        }
    }

    /**
     * One statement: read strictly, then refused as a whole, with every refusal its parts made
     * withdrawn, when the reader cannot express it or the type checker reported an error inside it
     * that no nested statement answered.
     */
    private List<Object> stmt(Statement s, Ctx ctx) {
        int mark = refusals.size();
        try {
            if (s.getStatementLabel() != null) {
                throw unsupported("labeled statement is outside cir/1");
            }
            List<Object> out = strict(s, ctx);
            for (Object o : out) {
                checkTyped(o);
            }
            Err e = firstError(ctx.unit, s);
            if (e != null) {
                throw refuse(DYNAMIC, e.msg);
            }
            return out;
        } catch (Unsupported u) {
            truncate(mark);
            consume(ctx.unit, s);
            return List.of(foreign(ctx.unit, s, u.getMessage(), ctx));
        }
    }

    private List<Object> strict(Statement s, Ctx ctx) {
        String pos = pos(ctx, s);
        if (s instanceof ExpressionStatement) {
            return exprStmt(((ExpressionStatement) s).getExpression(), ctx, pos);
        }
        if (s instanceof IfStatement) {
            IfStatement it = (IfStatement) s;
            Map<String, Object> cond = cond(it.getBooleanExpression(), ctx);
            Map<String, Object> st = Json.obj();
            st.put("k", "if");
            st.put("cond", cond);
            st.put("then", body(it.getIfBlock(), ctx));
            if (!(it.getElseBlock() instanceof EmptyStatement) && it.getElseBlock() != null) {
                st.put("else", body(it.getElseBlock(), ctx));
            }
            st.put("pos", pos);
            return List.of(st);
        }
        if (s instanceof WhileStatement) {
            WhileStatement wt = (WhileStatement) s;
            Map<String, Object> cond = cond(wt.getBooleanExpression(), ctx);
            Map<String, Object> st = Json.obj();
            st.put("k", "for");
            if (!isTrue(cond)) {
                st.put("cond", cond);
            }
            st.put("body", inside("loop", wt.getLoopBlock(), ctx));
            st.put("pos", pos);
            return List.of(st);
        }
        if (s instanceof ForStatement) {
            return forLoop((ForStatement) s, ctx, pos);
        }
        if (s instanceof SwitchStatement) {
            return switchStmt((SwitchStatement) s, ctx, pos);
        }
        if (s instanceof ReturnStatement) {
            return returnStmt((ReturnStatement) s, ctx, pos);
        }
        if (s instanceof BreakStatement) {
            if (((BreakStatement) s).getLabel() != null) {
                throw unsupported("labeled break is outside cir/1");
            }
            if (!"loop".equals(ctx.breaks.peek())) {
                throw unsupported("break out of a switch other than at the end of a case: cir/1 break leaves a loop");
            }
            return List.of(simple("break", pos));
        }
        if (s instanceof ContinueStatement) {
            if (((ContinueStatement) s).getLabel() != null) {
                throw unsupported("labeled continue is outside cir/1");
            }
            return List.of(simple("continue", pos));
        }
        if (s instanceof EmptyStatement) {
            return List.of();
        }
        if (s instanceof BlockStatement) {
            throw unsupported("a nested block is outside cir/1");
        }
        if (s instanceof ThrowStatement) {
            throw unsupported("throw: exceptions are outside the reader's subset");
        }
        if (s instanceof TryCatchStatement) {
            throw unsupported("try: catching exceptions is outside cir/1");
        }
        if (s instanceof SynchronizedStatement) {
            throw unsupported("synchronized: threads and locks are outside cir/1");
        }
        if (s instanceof AssertStatement) {
            throw unsupported("assert: Groovy's power assert is outside cir/1");
        }
        throw unsupported(s.getClass().getSimpleName() + " is outside cir/1");
    }

    private static boolean isTrue(Map<String, Object> e) {
        return "lit".equals(e.get("k")) && "true".equals(e.get("value"));
    }

    private static Map<String, Object> simple(String k, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", k);
        st.put("pos", pos);
        return st;
    }

    /** A condition: a {@code boolean}; any other value is Groovy truth. */
    private Map<String, Object> cond(Expression e, Ctx ctx) {
        Expression inner = e instanceof BooleanExpression && !(e instanceof NotExpression) ? ((BooleanExpression) e).getExpression() : e;
        Map<String, Object> x = expr(inner, ctx);
        if (!type(x).equals(tBool())) {
            throw truth(type(x));
        }
        return x;
    }

    private static Unsupported truth(Map<String, Object> t) {
        return refuse(GROOVY, "Groovy truth: a value of type " + describe(t) + " as a condition is true when it is not null, empty or zero");
    }

    private List<Object> exprStmt(Expression e, Ctx ctx, String pos) {
        if (e instanceof DeclarationExpression) {
            return local((DeclarationExpression) e, ctx, pos);
        }
        if (e instanceof BinaryExpression) {
            BinaryExpression b = (BinaryExpression) e;
            int op = b.getOperation().getType();
            if (op == Types.EQUAL) {
                Map<String, Object> lhs = place(b.getLeftExpression(), ctx);
                Map<String, Object> rhs = coerce(value(b.getRightExpression(), type(lhs), ctx), type(lhs));
                return List.of(assign("", lhs, rhs, pos));
            }
            String cop = compoundOp(op);
            if (cop != null) {
                Map<String, Object> lhs = place(b.getLeftExpression(), ctx);
                Map<String, Object> rhs = expr(b.getRightExpression(), ctx);
                Map<String, Object> lt = type(lhs);
                if (lt.equals(tString())) {
                    if (!cop.equals("+=") || !type(rhs).equals(tString())) {
                        throw unsupported("+= of a value other than a String to a String");
                    }
                } else if (!isInt(lt)) {
                    throw unsupported(cop + " on a value of type " + describe(lt));
                } else if (cop.equals("/=")) {
                    throw refuse(GROOVY, "/= on an integer divides as BigDecimal in Groovy, then truncates");
                }
                return List.of(assign(cop, lhs, coerce(rhs, lt), pos));
            }
            throw unsupported("an expression statement other than an assignment or a call");
        }
        if (e instanceof PostfixExpression || e instanceof PrefixExpression) {
            Expression x = e instanceof PostfixExpression ? ((PostfixExpression) e).getExpression() : ((PrefixExpression) e).getExpression();
            int op = e instanceof PostfixExpression ? ((PostfixExpression) e).getOperation().getType()
                : ((PrefixExpression) e).getOperation().getType();
            Map<String, Object> p = place(x, ctx);
            if (!isInt(type(p))) {
                throw unsupported("++ or -- on a value of type " + describe(type(p)));
            }
            Map<String, Object> st = Json.obj();
            st.put("k", op == Types.PLUS_PLUS ? "incr" : "decr");
            st.put("x", p);
            st.put("pos", pos);
            return List.of(st);
        }
        if (e instanceof MethodCallExpression) {
            List<Object> lib = collectionWrite((MethodCallExpression) e, ctx, pos);
            if (lib != null) {
                return lib;
            }
        }
        if (e instanceof MethodCallExpression || e instanceof StaticMethodCallExpression) {
            Map<String, Object> st = Json.obj();
            st.put("k", "expr");
            st.put("x", expr(e, ctx));
            st.put("pos", pos);
            return List.of(st);
        }
        if (e instanceof ClosureExpression) {
            throw refuse(CLOSURE, "closure: functions as values are outside cir/1");
        }
        throw unsupported("expression statement " + e.getClass().getSimpleName() + " is outside cir/1");
    }

    private static String compoundOp(int op) {
        switch (op) {
            case Types.PLUS_EQUAL:
                return "+=";
            case Types.MINUS_EQUAL:
                return "-=";
            case Types.MULTIPLY_EQUAL:
                return "*=";
            case Types.DIVIDE_EQUAL:
                return "/=";
            case Types.MOD_EQUAL:
                return "%=";
            case Types.BITWISE_AND_EQUAL:
                return "&=";
            case Types.BITWISE_OR_EQUAL:
                return "|=";
            case Types.BITWISE_XOR_EQUAL:
                return "^=";
            default:
                return null;
        }
    }

    private List<Object> local(DeclarationExpression d, Ctx ctx, String pos) {
        if (d.isMultipleAssignmentDeclaration()) {
            throw unsupported("multiple assignment is outside cir/1");
        }
        VariableExpression v = d.getVariableExpression();
        String name = v.getName();
        Map<String, Object> t;
        if (v.isDynamicTyped()) {
            ClassNode inferred = v.getNodeMetaData(StaticTypesMarker.DECLARATION_INFERRED_TYPE);
            if (inferred == null) {
                inferred = v.getNodeMetaData(StaticTypesMarker.INFERRED_TYPE);
            }
            if (inferred == null || inferred.getName().equals("java.lang.Object")) {
                throw refuse(DYNAMIC, "def local " + name + " has no inferred type: it holds whatever it is given");
            }
            t = typeOf(inferred);
        } else {
            t = typeOf(v.getOriginType());
        }
        Expression init = d.getRightExpression();
        if (v.isDynamicTyped() && init != null && !(init instanceof EmptyExpression)) {
            Map<String, Object> it = null;
            try {
                ClassNode ic = inferred(init);
                it = ic == null ? null : typeOf(ic);
            } catch (Unsupported u) {
                // the initializer's own read refuses it
            }
            if (it != null && !it.equals(t)) {
                ctx.widened.add(v);
                throw refuse(DYNAMIC, "def local " + name + " holds a value of type " + describe(it)
                    + " first and a value of type " + describe(t) + " later: its type changes as it is assigned");
            }
        }
        if (init == null || init instanceof EmptyExpression) {
            if (!isInt(t) && !t.equals(tBool())) {
                throw refuse(TYPE, "local " + name + " holds null until assigned, which no cir/1 zero value is");
            }
            ctx.vars.put(v, t);
            return List.of(var(name, t, pos));
        }
        Map<String, Object> x = value(init, t, ctx);
        ctx.vars.put(v, t);
        if ("nil".equals(x.get("k"))) {
            return List.of(var(name, coerce(x, t).get("t"), pos));
        }
        Map<String, Object> st = Json.obj();
        st.put("k", "let");
        st.put("names", new ArrayList<Object>(List.of(name)));
        st.put("rhs", new ArrayList<Object>(List.of(coerce(x, t))));
        st.put("pos", pos);
        return List.of(st);
    }

    private static Map<String, Object> var(String name, Object t, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", "var");
        st.put("name", name);
        st.put("t", t);
        st.put("pos", pos);
        return st;
    }

    private static Map<String, Object> assign(String op, Map<String, Object> lhs, Map<String, Object> rhs, String pos) {
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

    /** An expression that can be assigned to: a local or a field. */
    private Map<String, Object> place(Expression e, Ctx ctx) {
        Map<String, Object> x = expr(e, ctx);
        String k = (String) x.get("k");
        if (!k.equals("var") && !k.equals("field")) {
            throw unsupported("assignment to something other than a local or a field");
        }
        if (k.equals("field")) {
            checkFieldWrite(e, ctx);
        }
        return x;
    }

    /**
     * A write to a field read as one: refused where Groovy does something else. Through a receiver
     * other than {@code this}, a property write calls the class's {@code set<Name>} when it declares
     * one; a write to a final property outside its own constructor throws at run time.
     */
    private void checkFieldWrite(Expression e, Ctx ctx) {
        FieldNode f = null;
        boolean viaThis = true;
        if (e instanceof PropertyExpression) {
            PropertyExpression pe = (PropertyExpression) e;
            Expression obj = pe.getObjectExpression();
            viaThis = isThis(obj);
            ClassNode c = viaThis ? ctx.fn.cls : inferred(obj);
            if (c != null) {
                f = c.redirect().getField(pe.getPropertyAsString());
            }
        } else if (e instanceof VariableExpression) {
            Variable a = ((VariableExpression) e).getAccessedVariable();
            f = a instanceof PropertyNode ? ((PropertyNode) a).getField() : a instanceof FieldNode ? (FieldNode) a : null;
        }
        if (f == null) {
            return;
        }
        ClassNode owner = f.getOwner();
        String name = f.getName();
        String cap = Character.toUpperCase(name.charAt(0)) + name.substring(1);
        if (!viaThis && !owner.getMethods("set" + cap).isEmpty()) {
            throw refuse(GROOVY, "a write to property " + name + " of " + owner.getNameWithoutPackage() + " calls its setter set"
                + cap + ", not the field");
        }
        PropertyNode p = owner.getProperty(name);
        boolean isFinal = Modifier.isFinal(f.getModifiers()) || p != null && Modifier.isFinal(p.getModifiers());
        boolean ownCtor = ctx.fn != null && ctx.fn.isCtor && viaThis && ctx.fn.cls != null
            && ctx.fn.cls.getName().equals(owner.getName());
        if (isFinal && !ownCtor) {
            throw refuse(GROOVY, "a write to final " + (p != null ? "property " : "field ") + name
                + " outside its constructor: Groovy throws ReadOnlyPropertyException");
        }
    }

    /**
     * Whether a method, or one it calls, may change a collection: a call of a collection's mutating
     * method, {@code <<}, an index assignment, or a call the type checker does not resolve. Read
     * methods and constructors of the package are followed; library calls are judged by their name.
     */
    private boolean mayMutate(MethodNode m, Set<MethodNode> seen) {
        if (m == null || m.getCode() == null || !seen.add(m)) {
            return false;
        }
        boolean[] found = {false};
        m.getCode().visit(new CodeVisitorSupport() {
            @Override
            public void visitMethodCallExpression(MethodCallExpression call) {
                MethodNode t = call.getNodeMetaData(StaticTypesMarker.DIRECT_METHOD_CALL_TARGET);
                if (t == null || MUTATORS.contains(call.getMethodAsString())) {
                    found[0] = true;
                } else if (t.getDeclaringClass().redirect().isPrimaryClassNode() && mayMutate(t, seen)) {
                    found[0] = true;
                }
                super.visitMethodCallExpression(call);
            }

            @Override
            public void visitStaticMethodCallExpression(StaticMethodCallExpression call) {
                MethodNode t = call.getNodeMetaData(StaticTypesMarker.DIRECT_METHOD_CALL_TARGET);
                List<MethodNode> ts = t != null ? List.of(t) : call.getOwnerType().getMethods(call.getMethod());
                for (MethodNode x : ts) {
                    if (mayMutate(x, seen)) {
                        found[0] = true;
                    }
                }
                super.visitStaticMethodCallExpression(call);
            }

            @Override
            public void visitConstructorCallExpression(ConstructorCallExpression call) {
                if (call.getType().redirect().isPrimaryClassNode()) {
                    for (ConstructorNode c : call.getType().redirect().getDeclaredConstructors()) {
                        if (mayMutate(c, seen)) {
                            found[0] = true;
                        }
                    }
                }
                super.visitConstructorCallExpression(call);
            }

            @Override
            public void visitBinaryExpression(BinaryExpression b) {
                int op = b.getOperation().getType();
                if (op == Types.LEFT_SHIFT || op == Types.LEFT_SHIFT_EQUAL
                    || op == Types.EQUAL && b.getLeftExpression() instanceof BinaryExpression
                        && ((BinaryExpression) b.getLeftExpression()).getOperation().getType() == Types.LEFT_SQUARE_BRACKET) {
                    found[0] = true;
                }
                super.visitBinaryExpression(b);
            }
        });
        return found[0];
    }

    private static final Set<String> MUTATORS = Set.of(
        "add", "addAll", "addFirst", "addLast", "put", "putAll", "putIfAbsent", "remove", "removeAll", "removeAt",
        "removeIf", "removeElement", "retainAll", "clear", "set", "push", "pop", "leftShift", "compute",
        "computeIfAbsent", "computeIfPresent", "merge", "replace", "replaceAll", "sort", "offer", "poll",
        "pollFirst", "pollLast", "removeFirst", "removeLast", "subList", "iterator", "listIterator", "entrySet",
        "keySet", "values");

    /** A call of a read function inside a for-each, refused when the function may change a collection. */
    private void checkCallInForEach(Fn fn, String name, Ctx ctx) {
        if (ctx.forEach > 0 && fn.node != null && mayMutate(fn.node, new HashSet<>())) {
            throw unsupported("call of " + name + " inside a for-each: it may change a collection while it is"
                + " iterated, where Groovy throws ConcurrentModificationException and for_range runs on");
        }
    }

    /** An expression where a {@code want} is expected: {@code new ArrayList/HashMap/TreeMap<>()} takes its type from there. */
    private Map<String, Object> value(Expression e, Map<String, Object> want, Ctx ctx) {
        if (e instanceof ConstructorCallExpression && isNewCollection(e, FRESH)) {
            String cls = ((ConstructorCallExpression) e).getType().getName();
            boolean list = cls.equals("java.util.ArrayList");
            if (list && "slice".equals(want.get("k")) || !list && "map".equals(want.get("k"))) {
                Map<String, Object> mk = Json.obj();
                mk.put("k", "make");
                mk.put("t", want);
                return mk;
            }
        }
        return expr(e, ctx);
    }

    private static final Set<String> FRESH = Set.of("java.util.ArrayList", "java.util.HashMap", "java.util.TreeMap");

    /** Whether {@code e} is {@code new C<>()} for one of {@code classes}, without arguments. */
    private static boolean isNewCollection(Expression e, Set<String> classes) {
        if (!(e instanceof ConstructorCallExpression)) {
            return false;
        }
        ConstructorCallExpression c = (ConstructorCallExpression) e;
        return !c.isSpecialCall() && !c.isUsingAnonymousInnerClass() && args(c.getArguments()).isEmpty()
            && classes.contains(c.getType().getName());
    }

    private static List<Expression> args(Expression a) {
        if (a instanceof TupleExpression) {
            return ((TupleExpression) a).getExpressions();
        }
        return List.of(a);
    }

    // ---- collections no other name shares ----

    /** The local, parameter or field an expression names, or null when it computes a value. */
    private static Object named(Expression e) {
        if (e instanceof VariableExpression && !((VariableExpression) e).isThisExpression()) {
            Variable v = ((VariableExpression) e).getAccessedVariable();
            if (v instanceof PropertyNode) {
                return ((PropertyNode) v).getField();
            }
            return v instanceof DynamicVariable ? null : v;
        }
        if (e instanceof PropertyExpression && !(e instanceof AttributeExpression) && isThis(((PropertyExpression) e).getObjectExpression())) {
            ClassNode cls = ((PropertyExpression) e).getObjectExpression().getType();
            String n = ((PropertyExpression) e).getPropertyAsString();
            return n == null ? null : cls.getField(n);
        }
        return null;
    }

    /** Whether an expression names {@code target}: a variable, or a field by name or as {@code x.f} of its class. */
    private boolean names(Expression e, Object target) {
        if (e instanceof VariableExpression) {
            Variable v = ((VariableExpression) e).getAccessedVariable();
            Object o = v instanceof PropertyNode ? ((PropertyNode) v).getField() : v;
            return o == target;
        }
        if (target instanceof FieldNode && e instanceof PropertyExpression) {
            FieldNode f = (FieldNode) target;
            PropertyExpression pe = (PropertyExpression) e;
            if (!f.getName().equals(pe.getPropertyAsString())) {
                return false;
            }
            Expression o = pe.getObjectExpression();
            if (isThis(o)) {
                return true;
            }
            ClassNode t = chooser.resolveType(o, f.getOwner());
            return t != null && t.getName().equals(f.getOwner().getName());
        }
        return false;
    }

    /**
     * The uses of {@code target} under {@code root}: every use, those that leave the collection with
     * this name only (a receiver, the collection of a for-each, and with {@code returned} a return
     * value), and every assignment, its initializer included, with whether it is {@code new
     * ArrayList/HashMap/TreeMap<>()}.
     */
    private int[] uses(ASTNode root, Object target, boolean returned, Set<String> fresh) {
        int[] c = {0, 0, 0, 1};
        Predicate<Expression> is = e -> names(e, target);
        CodeVisitorSupport v = new CodeVisitorSupport() {
            @Override
            public void visitVariableExpression(VariableExpression e) {
                if (is.test(e)) {
                    c[0]++;
                }
            }

            @Override
            public void visitPropertyExpression(PropertyExpression e) {
                if (is.test(e)) {
                    c[0]++;
                    return;
                }
                super.visitPropertyExpression(e);
            }

            @Override
            public void visitMethodCallExpression(MethodCallExpression e) {
                if (is.test(e.getObjectExpression())) {
                    c[1]++;
                }
                super.visitMethodCallExpression(e);
            }

            @Override
            public void visitForLoop(ForStatement s) {
                if (is.test(s.getCollectionExpression())) {
                    c[1]++;
                }
                super.visitForLoop(s);
            }

            @Override
            public void visitReturnStatement(ReturnStatement s) {
                if (returned && is.test(s.getExpression())) {
                    c[1]++;
                }
                super.visitReturnStatement(s);
            }

            @Override
            public void visitBinaryExpression(BinaryExpression e) {
                if (e.getOperation().getType() == Types.EQUAL && is.test(e.getLeftExpression())) {
                    c[1]++;
                    c[2]++;
                    if (!isNewCollection(e.getRightExpression(), fresh)) {
                        c[3] = 0;
                    }
                }
                super.visitBinaryExpression(e);
            }
        };
        root.visit(v);
        return c;
    }

    /** The code of every read class: field initializers and method bodies. */
    private List<ASTNode> everyBody() {
        List<ASTNode> out = new ArrayList<>();
        for (ClassNode cn : decls) {
            for (MethodNode m : userMethods(cn)) {
                if (m.getCode() != null) {
                    out.add(m.getCode());
                }
            }
            for (FieldNode f : userFields(cn)) {
                if (f.hasInitialExpression()) {
                    out.add(f.getInitialExpression());
                }
            }
        }
        return out;
    }

    /**
     * Whether a field's collection is reached by no name but the field, in every read file: its
     * initializer and every assignment are {@code new ArrayList/HashMap/TreeMap<>()}, and every other
     * use is as a receiver or as the collection of a for-each.
     */
    private boolean ownedField(FieldNode f, Set<String> fresh) {
        int all = 0;
        int ok = 0;
        int assigned = 0;
        boolean allFresh = true;
        if (f.hasInitialExpression()) {
            assigned++;
            allFresh &= isNewCollection(f.getInitialExpression(), fresh);
        }
        for (ASTNode b : everyBody()) {
            int[] c = uses(b, f, false, fresh);
            all += c[0];
            ok += c[1];
            assigned += c[2];
            allFresh &= c[3] == 1;
        }
        return all == ok && assigned > 0 && allFresh;
    }

    /**
     * Whether a collection is one only this name reaches: a field of {@code this} {@link
     * #ownedField}, or a local initialised with {@code new ...<>()} whose every other use is as a
     * receiver, the collection of a for-each or the value of a return.
     */
    private boolean owned(Expression recv, Ctx ctx) {
        Object n = named(recv);
        if (n instanceof FieldNode) {
            FieldNode f = (FieldNode) n;
            return !f.isStatic() && (recv instanceof VariableExpression || isThis(((PropertyExpression) recv).getObjectExpression()))
                && ownedField(f, FRESH);
        }
        if (!(n instanceof VariableExpression) || ctx.fn.node == null) {
            return false;
        }
        int[] c = uses(ctx.fn.node.getCode(), n, true, FRESH);
        return c[0] == c[1] && c[2] > 0 && c[3] == 1;
    }

    /**
     * Whether a map is a local or an instance field every assignment of which, its initializer
     * included, is {@code new TreeMap<>()}: a TreeMap with the keys' natural order.
     */
    private boolean naturalTreeMap(Expression recv, Ctx ctx) {
        Object n = named(recv);
        Set<String> tree = Set.of("java.util.TreeMap");
        if (n instanceof FieldNode && !((FieldNode) n).isStatic()) {
            FieldNode f = (FieldNode) n;
            int assigned = 0;
            boolean ok = true;
            if (f.hasInitialExpression()) {
                assigned++;
                ok &= isNewCollection(f.getInitialExpression(), tree);
            }
            for (ASTNode b : everyBody()) {
                int[] c = uses(b, f, false, tree);
                assigned += c[2];
                ok &= c[3] == 1;
            }
            return ok && assigned > 0;
        }
        if (n instanceof VariableExpression && ctx.fn.node != null) {
            int[] c = uses(ctx.fn.node.getCode(), n, true, tree);
            return c[2] > 0 && c[3] == 1;
        }
        return false;
    }

    /** {@code list.add(x)}, {@code list.set(i, x)} and {@code map.put(k, v)} as statements. */
    private List<Object> collectionWrite(MethodCallExpression mce, Ctx ctx, String pos) {
        MethodNode m = mce.getNodeMetaData(StaticTypesMarker.DIRECT_METHOD_CALL_TARGET);
        if (m == null || mce.isImplicitThis() || mce.isSafe() || mce.isSpreadSafe() || m instanceof ExtensionMethodNode
            || !m.getDeclaringClass().getName().startsWith("java.util.")) {
            return null;
        }
        String name = m.getName();
        List<Expression> as = args(mce.getArguments());
        int n = as.size();
        boolean list = name.equals("add") && n == 1 || name.equals("set") && n == 2;
        boolean map = name.equals("put") && n == 2;
        if (!list && !map) {
            return null;
        }
        Expression recvExpr = mce.getObjectExpression();
        if (!owned(recvExpr, ctx)) {
            throw unsupported(name + " on a collection another name may share: a Groovy collection is one object every"
                + " name sees changed, a cir/1 slice or map write is not; only a field of this or a local built here and"
                + " never handed on is written");
        }
        Object target = named(recvExpr);
        if (target != null && ctx.iterating.contains(target)) {
            throw unsupported(name + " on the collection a for-each is iterating: Groovy throws"
                + " ConcurrentModificationException, a cir/1 range runs on");
        }
        Map<String, Object> recv = expr(recvExpr, ctx);
        Map<String, Object> rt = type(recv);
        if (list && "slice".equals(rt.get("k"))) {
            Map<String, Object> elem = sub(rt, "elem");
            if (name.equals("add")) {
                Map<String, Object> p = place(recvExpr, ctx);
                Map<String, Object> x = coerce(expr(as.get(0), ctx), elem);
                return List.of(assign("", p, intrinsic("append", List.of(recv, x), rt), pos));
            }
            Map<String, Object> i = coerce(expr(as.get(0), ctx), tInt("32"));
            Map<String, Object> x = coerce(expr(as.get(1), ctx), elem);
            return List.of(assign("", index(recv, i, elem), x, pos));
        }
        if (map && "map".equals(rt.get("k"))) {
            Map<String, Object> k = coerce(expr(as.get(0), ctx), sub(rt, "key"));
            Map<String, Object> v = coerce(expr(as.get(1), ctx), sub(rt, "elem"));
            return List.of(assign("", index(recv, k, sub(rt, "elem")), v, pos));
        }
        return null;
    }

    // ---- loops, switch, return ----

    private List<Object> forLoop(ForStatement fs, Ctx ctx, String pos) {
        if (fs.getVariable() == ForStatement.FOR_LOOP_DUMMY) {
            return classicFor(fs, ctx, pos);
        }
        Parameter var = fs.getVariable();
        String name = var.getName();
        Expression coll = fs.getCollectionExpression();
        String key = "_";
        String val = name;
        Map<String, Object> over;
        Map<String, Object> vt;
        Object source;
        if (coll instanceof MethodCallExpression && "keySet".equals(((MethodCallExpression) coll).getMethodAsString())
            && args(((MethodCallExpression) coll).getArguments()).isEmpty() && !((MethodCallExpression) coll).isSafe()
            && !((MethodCallExpression) coll).isImplicitThis()) {
            Expression mp = ((MethodCallExpression) coll).getObjectExpression();
            source = named(mp);
            String cls = staticType(mp, ctx).getName();
            Map<String, Object> m = expr(mp, ctx);
            if (!"map".equals(type(m).get("k"))) {
                throw unsupported("keySet of something other than a map");
            }
            Map<String, Object> kt = sub(type(m), "key");
            vt = loopVar(var, kt);
            if (cls.equals("java.util.HashMap")) {
                key = name;
                val = "_";
                over = m;
            } else if (cls.equals("java.util.TreeMap")) {
                if (!kt.equals(tString()) && !isInt(kt)) {
                    throw unsupported("a TreeMap over " + describe(kt) + " keys iterates in their compareTo order;"
                        + " sorted_keys orders only strings and integers");
                }
                if (!naturalTreeMap(mp, ctx)) {
                    throw unsupported("a TreeMap the reader did not see built with new TreeMap<>() may carry any"
                        + " comparator; sorted_keys is the natural order");
                }
                over = intrinsic("sorted_keys", List.of(m), tSlice(kt));
            } else {
                throw unsupported("iteration over a " + cls + ": its order depends on the class behind it; declare it"
                    + " HashMap (unordered) or TreeMap (sorted)");
            }
        } else {
            over = expr(coll, ctx);
            source = named(coll);
            String cls = staticType(coll, ctx).getName();
            if (!"slice".equals(type(over).get("k")) || !(cls.equals("java.util.List") || cls.equals("java.util.ArrayList"))) {
                throw unsupported("a for loop over something other than a List or a map's keySet()");
            }
            vt = loopVar(var, sub(type(over), "elem"));
        }
        ctx.vars.put(var, vt);
        ctx.used.add(name);
        Map<String, Object> st = Json.obj();
        st.put("k", "for_range");
        st.put("key", key);
        st.put("val", val);
        st.put("over", over);
        if (source != null) {
            ctx.iterating.push(source);
        }
        ctx.forEach++;
        try {
            st.put("body", inside("loop", fs.getLoopBlock(), ctx));
        } finally {
            ctx.forEach--;
            if (source != null) {
                ctx.iterating.pop();
            }
        }
        st.put("pos", pos);
        return List.of(st);
    }

    /** The loop variable's type: the element's for {@code for (x in c)}, else its own, which must be the element's. */
    private Map<String, Object> loopVar(Parameter var, Map<String, Object> elem) {
        if (var.isDynamicTyped()) {
            return elem;
        }
        Map<String, Object> vt = typeOf(var.getOriginType());
        if (!vt.equals(elem)) {
            throw unsupported("a loop variable of a type other than the element's");
        }
        return vt;
    }

    private List<Object> classicFor(ForStatement fs, Ctx ctx, String pos) {
        if (!(fs.getCollectionExpression() instanceof ClosureListExpression)) {
            throw unsupported("a for loop of a form outside cir/1");
        }
        List<Expression> parts = ((ClosureListExpression) fs.getCollectionExpression()).getExpressions();
        if (parts.size() != 3) {
            throw unsupported("for with more than one init or update expression");
        }
        Map<String, Object> st = Json.obj();
        st.put("k", "for");
        if (!(parts.get(0) instanceof EmptyExpression)) {
            if (parts.get(0) instanceof ClosureListExpression) {
                throw unsupported("for with more than one init expression");
            }
            List<Object> init = exprStmt(parts.get(0), ctx, pos);
            String k = init.size() == 1 ? (String) ((Map<?, ?>) init.get(0)).get("k") : "";
            if (!k.equals("let") && !k.equals("assign")) {
                throw unsupported("for with an init statement other than a declaration or an assignment");
            }
            st.put("init", init.get(0));
        }
        if (!(parts.get(1) instanceof EmptyExpression)) {
            Map<String, Object> c = cond(parts.get(1), ctx);
            if (!isTrue(c)) {
                st.put("cond", c);
            }
        }
        if (!(parts.get(2) instanceof EmptyExpression)) {
            if (parts.get(2) instanceof ClosureListExpression) {
                throw unsupported("for with more than one update expression");
            }
            List<Object> u = exprStmt(parts.get(2), ctx, pos);
            String k = u.size() == 1 ? (String) ((Map<?, ?>) u.get(0)).get("k") : "";
            if (!k.equals("assign") && !k.equals("incr") && !k.equals("decr")) {
                throw unsupported("for with an update other than an assignment, ++ or --");
            }
            st.put("post", u.get(0));
        }
        st.put("body", inside("loop", fs.getLoopBlock(), ctx));
        st.put("pos", pos);
        return List.of(st);
    }

    private List<Object> switchStmt(SwitchStatement sw, Ctx ctx, String pos) {
        Map<String, Object> tag = expr(sw.getExpression(), ctx);
        Map<String, Object> tt = type(tag);
        if (!isInt(tt) && !tt.equals(tString()) && enumName(tt) == null) {
            throw unsupported("switch on a value of type " + describe(tt));
        }
        List<List<Object>> values = new ArrayList<>();
        List<List<Statement>> bodies = new ArrayList<>();
        List<Object> pending = new ArrayList<>();
        List<CaseStatement> cases = sw.getCaseStatements();
        List<Statement> dflt = statements(sw.getDefaultStatement());
        boolean hasDefault = !dflt.isEmpty();
        for (int i = 0; i < cases.size(); i++) {
            CaseStatement c = cases.get(i);
            pending.add(caseValue(c.getExpression(), tt, ctx));
            List<Statement> stmts = statements(c.getCode());
            boolean lastCase = i == cases.size() - 1;
            if (stmts.isEmpty() && !lastCase) {
                continue;
            }
            if (stmts.isEmpty() && hasDefault) {
                throw unsupported("a case label sharing its body with default");
            }
            if (!stmts.isEmpty()) {
                Statement end = stmts.get(stmts.size() - 1);
                if (end instanceof BreakStatement && ((BreakStatement) end).getLabel() == null) {
                    stmts = stmts.subList(0, stmts.size() - 1);
                } else if (!exits(end) && !(end instanceof ContinueStatement) && !(lastCase && !hasDefault)) {
                    throw unsupported("a case falling through to the next is outside cir/1");
                }
            }
            values.add(pending);
            bodies.add(stmts);
            pending = new ArrayList<>();
        }
        if (hasDefault) {
            Statement end = dflt.get(dflt.size() - 1);
            if (end instanceof BreakStatement && ((BreakStatement) end).getLabel() == null) {
                dflt = dflt.subList(0, dflt.size() - 1);
            }
            values.add(new ArrayList<>());
            bodies.add(dflt);
        }
        List<Object> outCases = new ArrayList<>();
        for (int i = 0; i < values.size(); i++) {
            ctx.breaks.push("switch");
            try {
                Map<String, Object> c = Json.obj();
                c.put("values", values.get(i));
                List<Object> b = new ArrayList<>();
                for (Statement s : bodies.get(i)) {
                    b.addAll(stmt(s, ctx));
                }
                c.put("body", b);
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

    /** A case label: a String or integer literal, or a constant of the enum switched on; anything else calls isCase. */
    private Map<String, Object> caseValue(Expression e, Map<String, Object> tt, Ctx ctx) {
        boolean literal = e instanceof ConstantExpression && ((ConstantExpression) e).getValue() != null;
        boolean enumConst = e instanceof PropertyExpression && ((PropertyExpression) e).getObjectExpression() instanceof ClassExpression;
        if (!literal && !enumConst) {
            throw refuse(GROOVY, "case " + e.getText() + " calls isCase, which matches by the label's own rule");
        }
        Map<String, Object> v = coerce(expr(e, ctx), tt);
        if (!"lit".equals(v.get("k")) && !"const".equals(v.get("k"))) {
            throw refuse(GROOVY, "case " + e.getText() + " calls isCase");
        }
        return v;
    }

    private List<Object> returnStmt(ReturnStatement rs, Ctx ctx, String pos) {
        Fn fn = ctx.fn;
        if (fn.isCtor || fn.result == null) {
            if (!rs.isReturningNullOrVoid()) {
                throw unsupported("a value returned from a method without a result");
            }
            return List.of(ret(successResults(ctx, null), pos));
        }
        Map<String, Object> x = coerce(value(rs.getExpression(), fn.result, ctx), fn.result);
        return List.of(ret(successResults(ctx, x), pos));
    }

    private List<Object> successResults(Ctx ctx, Map<String, Object> value) {
        List<Object> rs = new ArrayList<>();
        if (ctx.fn.isCtor) {
            rs.add(varRef("this", tPtr(tNamed(ctx.fn.cls.getNameWithoutPackage()))));
        } else if (value != null) {
            rs.add(value);
        }
        return rs;
    }

    private static Map<String, Object> ret(List<?> results, String pos) {
        Map<String, Object> st = Json.obj();
        st.put("k", "return");
        st.put("results", new ArrayList<Object>(results));
        st.put("pos", pos);
        return st;
    }

    // ---- expressions ----

    private Map<String, Object> expr(Expression e, Ctx ctx) {
        if (e instanceof ClosureExpression) {
            throw refuse(CLOSURE, "closure: functions as values are outside cir/1");
        }
        if (e instanceof MethodPointerExpression) {
            throw refuse(CLOSURE, "method pointer .&: functions as values are outside cir/1");
        }
        if (e instanceof ConstantExpression) {
            return constant((ConstantExpression) e);
        }
        if (e instanceof GStringExpression) {
            return gstring((GStringExpression) e, ctx);
        }
        if (e instanceof VariableExpression) {
            return variable((VariableExpression) e, ctx);
        }
        if (e instanceof AttributeExpression) {
            throw unsupported(".@ reads a field around its accessor, which is outside the reader's subset");
        }
        if (e instanceof PropertyExpression) {
            return property((PropertyExpression) e, ctx);
        }
        if (e instanceof MethodCallExpression) {
            return call((MethodCallExpression) e, ctx);
        }
        if (e instanceof StaticMethodCallExpression) {
            return staticCall((StaticMethodCallExpression) e, ctx);
        }
        if (e instanceof ConstructorCallExpression) {
            return newObject((ConstructorCallExpression) e, ctx);
        }
        if (e instanceof DeclarationExpression) {
            throw unsupported("a declaration inside an expression");
        }
        if (e instanceof BinaryExpression) {
            return binary((BinaryExpression) e, ctx);
        }
        if (e instanceof NotExpression) {
            Map<String, Object> x = expr(((NotExpression) e).getExpression(), ctx);
            if (!type(x).equals(tBool())) {
                throw truth(type(x));
            }
            return unop("!", x, tBool());
        }
        if (e instanceof BooleanExpression) {
            return cond(e, ctx);
        }
        if (e instanceof ElvisOperatorExpression) {
            throw refuse(GROOVY, "?: gives the right side when the left is null, empty or zero (Groovy truth)");
        }
        if (e instanceof TernaryExpression) {
            throw unsupported("c ? a : b is outside cir/1");
        }
        if (e instanceof UnaryMinusExpression || e instanceof UnaryPlusExpression || e instanceof BitwiseNegationExpression) {
            Expression inner = e instanceof UnaryMinusExpression ? ((UnaryMinusExpression) e).getExpression()
                : e instanceof UnaryPlusExpression ? ((UnaryPlusExpression) e).getExpression()
                : ((BitwiseNegationExpression) e).getExpression();
            Map<String, Object> x = expr(inner, ctx);
            if (!isInt(type(x))) {
                throw unsupported("unary operator on a value of type " + describe(type(x)));
            }
            String op = e instanceof UnaryMinusExpression ? "-" : e instanceof UnaryPlusExpression ? "+" : "^";
            return unop(op, x, type(x));
        }
        if (e instanceof PostfixExpression || e instanceof PrefixExpression) {
            throw unsupported("++ or -- inside an expression");
        }
        if (e instanceof CastExpression) {
            if (((CastExpression) e).isCoerce()) {
                throw refuse(GROOVY, "as converts by Groovy's asType rules");
            }
            throw unsupported("cast: conversions are outside the reader's subset");
        }
        if (e instanceof RangeExpression) {
            throw refuse(GROOVY, "range: a Groovy range is outside cir/1");
        }
        if (e instanceof SpreadExpression || e instanceof SpreadMapExpression) {
            throw refuse(GROOVY, "spread *: is outside cir/1");
        }
        if (e instanceof ListExpression) {
            throw unsupported("list literal: the reader reads new ArrayList<>() only");
        }
        if (e instanceof MapExpression) {
            throw unsupported("map literal: a Groovy map literal is a LinkedHashMap, ordered by insertion");
        }
        if (e instanceof ClassExpression) {
            throw unsupported("a class as a value: reflection is outside cir/1");
        }
        if (e instanceof FieldExpression) {
            return fieldRef(((FieldExpression) e).getField(), null, ctx);
        }
        if (e instanceof ArrayExpression) {
            throw refuse(TYPE, JvmTypes.outside(text(((ArrayExpression) e).getType())));
        }
        throw unsupported(e.getClass().getSimpleName() + " is outside cir/1");
    }

    private Map<String, Object> constant(ConstantExpression c) {
        Object v = c.getValue();
        if (v == null) {
            return nil(NULL);
        }
        if (v instanceof Integer) {
            return lit(String.valueOf(v), tInt("32"));
        }
        if (v instanceof Long) {
            return lit(String.valueOf(v), tInt("64"));
        }
        if (v instanceof Short) {
            return lit(String.valueOf(v), tInt("16"));
        }
        if (v instanceof Byte) {
            return lit(String.valueOf(v), tInt("8"));
        }
        if (v instanceof Boolean) {
            return lit(String.valueOf(v), tBool());
        }
        if (v instanceof String) {
            return lit(quote((String) v), tString());
        }
        typeOf(ClassHelper.make(v.getClass()));
        throw refuse(TYPE, JvmTypes.outside(v.getClass().getName()));
    }

    /** A GString of String and integer values: {@code sprintf} with {@code %s} and {@code %d}. */
    private Map<String, Object> gstring(GStringExpression g, Ctx ctx) {
        List<ConstantExpression> strings = g.getStrings();
        List<Expression> values = g.getValues();
        StringBuilder fmt = new StringBuilder();
        List<Object> args = new ArrayList<>();
        args.add(null);
        for (int i = 0; i < strings.size() || i < values.size(); i++) {
            if (i < strings.size()) {
                fmt.append(String.valueOf(strings.get(i).getValue()).replace("%", "%%"));
            }
            if (i < values.size()) {
                Map<String, Object> x = expr(values.get(i), ctx);
                Map<String, Object> t = type(x);
                if (t.equals(tString())) {
                    fmt.append("%s");
                } else if (isInt(t)) {
                    fmt.append("%d");
                } else {
                    throw refuse(GROOVY, "a GString value of type " + describe(t)
                        + ": only String and integer values are read, as Groovy prints them with toString");
                }
                args.add(x);
            }
        }
        args.set(0, lit(quote(fmt.toString()), tString()));
        return intrinsic("sprintf", args, tString());
    }

    private Map<String, Object> thisRef(Ctx ctx) {
        if (!ctx.hasThis) {
            throw unsupported("this where there is no object: a static method or a field initializer");
        }
        return varRef("this", tPtr(tNamed(ctx.fn.cls.getNameWithoutPackage())));
    }

    private Map<String, Object> variable(VariableExpression v, Ctx ctx) {
        if (v.isThisExpression()) {
            return thisRef(ctx);
        }
        if (v.isSuperExpression()) {
            throw unsupported("super: inheritance is outside cir/1");
        }
        Variable a = v.getAccessedVariable();
        if (a == null || a instanceof DynamicVariable) {
            throw refuse(DYNAMIC, "name " + v.getName() + " does not resolve: Groovy looks it up at run time");
        }
        if (a instanceof PropertyNode) {
            return fieldRef(((PropertyNode) a).getField(), null, ctx);
        }
        if (a instanceof FieldNode) {
            return fieldRef((FieldNode) a, null, ctx);
        }
        if (ctx.widened.contains(a)) {
            throw refuse(DYNAMIC, "def local " + v.getName() + " changes its type as it is assigned");
        }
        Map<String, Object> t = ctx.vars.get(a);
        if (t == null) {
            // A local whose declaration was kept as foreign: typed from that declaration, as the
            // Java reader types every local from its symbol.
            t = declaredType(a, v.getName());
        }
        return varRef(v.getName(), t);
    }

    private Map<String, Object> declaredType(Variable a, String name) {
        if (!a.isDynamicTyped()) {
            return typeOf(a.getOriginType());
        }
        ClassNode inferred = a instanceof ASTNode ? ((ASTNode) a).getNodeMetaData(StaticTypesMarker.DECLARATION_INFERRED_TYPE) : null;
        if (inferred == null || inferred.getName().equals("java.lang.Object")) {
            throw refuse(DYNAMIC, "def local " + name + " has no inferred type: it holds whatever it is given");
        }
        return typeOf(inferred);
    }

    private Map<String, Object> fieldRef(FieldNode f, Map<String, Object> recv, Ctx ctx) {
        ClassNode owner = f.getOwner();
        String name = f.getName();
        if (f.isStatic()) {
            if ((f.getModifiers() & ACC_ENUM) != 0) {
                if (!enums.containsKey(owner.getName())) {
                    throw unsupported("constant of enum " + owner.getName() + ", which is not read");
                }
                return constRef(name, tNamed(owner.getNameWithoutPackage()));
            }
            if (consts.containsKey(f)) {
                return constRef(consts.get(f), typeOf(f.getType()));
            }
            Map<String, Object> lib = libraryConst(owner, name);
            if (lib != null) {
                return lib;
            }
            throw unsupported("static field " + owner.getNameWithoutPackage() + "." + name);
        }
        if (!structs.containsKey(owner.getName())) {
            throw unsupported("field of " + owner.getName() + ", which is not read");
        }
        Map<String, Object> x = recv != null ? recv : thisRef(ctx);
        Map<String, Object> fm = Json.obj();
        fm.put("k", "field");
        fm.put("x", x);
        fm.put("name", name);
        fm.put("t", typeOf(f.getType()));
        return fm;
    }

    /** {@code Integer.MAX_VALUE} and the other integer constants of {@code Integer} and {@code Long}. */
    private static Map<String, Object> libraryConst(ClassNode owner, String name) {
        String on = owner.getName();
        if (!on.equals("java.lang.Integer") && !on.equals("java.lang.Long")) {
            return null;
        }
        try {
            java.lang.reflect.Field f = Class.forName(on).getField(name);
            if (!Modifier.isStatic(f.getModifiers()) || !Modifier.isFinal(f.getModifiers())) {
                return null;
            }
            Object v = f.get(null);
            if (v instanceof Integer) {
                return lit(String.valueOf(v), tInt("32"));
            }
            if (v instanceof Long) {
                return lit(String.valueOf(v), tInt("64"));
            }
            return null;
        } catch (ReflectiveOperationException e) {
            return null;
        }
    }

    /** Whether an expression reaches a class's metaclass. */
    private static boolean metaClass(Expression e) {
        while (true) {
            if (e instanceof PropertyExpression) {
                if ("metaClass".equals(((PropertyExpression) e).getPropertyAsString())) {
                    return true;
                }
                e = ((PropertyExpression) e).getObjectExpression();
            } else if (e instanceof MethodCallExpression) {
                if ("getMetaClass".equals(((MethodCallExpression) e).getMethodAsString())) {
                    return true;
                }
                e = ((MethodCallExpression) e).getObjectExpression();
            } else if (e instanceof VariableExpression) {
                return "metaClass".equals(((VariableExpression) e).getName());
            } else {
                return false;
            }
        }
    }

    private Map<String, Object> property(PropertyExpression pe, Ctx ctx) {
        if (metaClass(pe)) {
            throw refuse(DYNAMIC, "metaClass: a class changed through its metaclass at run time");
        }
        if (pe.isSpreadSafe()) {
            throw refuse(GROOVY, "spread *. is outside cir/1");
        }
        if (pe.isSafe()) {
            throw refuse(GROOVY, "?. gives null when the receiver is null");
        }
        String name = pe.getPropertyAsString();
        if (name == null) {
            throw refuse(DYNAMIC, "a property named at run time");
        }
        Expression obj = pe.getObjectExpression();
        if (obj instanceof ClassExpression) {
            ClassNode c = obj.getType();
            FieldNode f = c.getField(name);
            if (f != null && f.isStatic() && (enums.containsKey(c.getName()) || structs.containsKey(c.getName()))) {
                return fieldRef(f, null, ctx);
            }
            Map<String, Object> lib = libraryConst(c, name);
            if (lib != null) {
                return lib;
            }
            throw unsupported("static property " + c.getNameWithoutPackage() + "." + name + " is outside the reader's subset");
        }
        Map<String, Object> recv = isThis(obj) ? thisRef(ctx) : expr(obj, ctx);
        Map<String, Object> rt = type(recv);
        if ("ptr".equals(rt.get("k")) && "named".equals(sub(rt, "elem").get("k"))) {
            ClassNode s = structByName((String) sub(rt, "elem").get("name"));
            FieldNode f = s == null ? null : s.getField(name);
            if (f != null && !f.isStatic() && userNode(f)) {
                String cap = Character.toUpperCase(name.charAt(0)) + name.substring(1);
                if (!s.getMethods("get" + cap).isEmpty() || !s.getMethods("is" + cap).isEmpty()) {
                    throw unsupported("property " + name + " of " + s.getNameWithoutPackage()
                        + " has an accessor of its own, which Groovy calls");
                }
                return fieldRef(f, recv, ctx);
            }
        }
        throw unsupported("property " + name + " of a value of type " + describe(rt) + " is outside the reader's subset");
    }

    private ClassNode structByName(String simple) {
        for (ClassNode c : structs.values()) {
            if (c.getNameWithoutPackage().equals(simple)) {
                return c;
            }
        }
        return null;
    }

    /** A method call, read by the target the type checker chose. */
    private Map<String, Object> call(MethodCallExpression mce, Ctx ctx) {
        if (metaClass(mce)) {
            throw refuse(DYNAMIC, "metaClass: a class changed through its metaclass at run time");
        }
        if (mce.isSpreadSafe()) {
            throw refuse(GROOVY, "spread *. is outside cir/1");
        }
        if (mce.isSafe()) {
            throw refuse(GROOVY, "?. gives null when the receiver is null");
        }
        String name = mce.getMethodAsString();
        if (name == null) {
            throw refuse(DYNAMIC, "a method named at run time");
        }
        MethodNode target = mce.getNodeMetaData(StaticTypesMarker.DIRECT_METHOD_CALL_TARGET);
        if (target == null) {
            throw refuse(DYNAMIC, "call of " + name + ", which the type checker does not resolve: Groovy looks it up at run time");
        }
        List<Expression> as = args(mce.getArguments());
        for (Expression a : as) {
            if (a instanceof ClosureExpression || a instanceof MethodPointerExpression) {
                throw refuse(CLOSURE, "a closure passed to " + name + ": functions as values are outside cir/1");
            }
            if (a instanceof MapExpression) {
                throw refuse(GROOVY, "named arguments to " + name + " are passed as a map");
            }
        }
        if (target instanceof ExtensionMethodNode) {
            throw unsupported("Groovy extension method " + name + " is outside the reader's library table");
        }
        ClassNode owner = target.getDeclaringClass();
        String on = owner.getName();
        if (name.equals("getClass") || on.equals("java.lang.Class") || on.startsWith("java.lang.reflect.")) {
            throw unsupported("reflection: " + owner.getNameWithoutPackage() + "." + name + " is outside cir/1");
        }
        Expression recvExpr = mce.getObjectExpression();
        Fn fn = fns.get(target);
        if (fn != null) {
            checkCallInForEach(fn, name, ctx);
            List<Object> argv = argValues(as, fn.params, ctx);
            Map<String, Object> c = Json.obj();
            if (fn.isStatic) {
                if (!mce.isImplicitThis() && !(recvExpr instanceof ClassExpression)) {
                    throw unsupported("a static method called through an expression, which Groovy evaluates and then ignores");
                }
                c.put("k", "call");
                c.put("name", fn.name);
            } else {
                c.put("k", "method");
                c.put("x", mce.isImplicitThis() || isThis(recvExpr) ? thisRef(ctx) : expr(recvExpr, ctx));
                c.put("name", fn.name);
            }
            c.put("args", argv);
            c.put("t", fn.resultType());
            return c;
        }
        if (owner.redirect().isPrimaryClassNode()) {
            throw unsupported("call of " + owner.getNameWithoutPackage() + "." + name + ", which is not read");
        }
        if (on.equals("java.lang.String") && name.equals("format") && target.isStatic()) {
            return format(as, ctx);
        }
        if (mce.isImplicitThis() || !on.startsWith("java.")) {
            throw unsupported(owner.getNameWithoutPackage() + "." + name + " is outside the reader's library table");
        }
        Map<String, Object> recv = expr(recvExpr, ctx);
        Map<String, Object> rt = type(recv);
        String rk = (String) rt.get("k");
        int n = as.size();
        if ("slice".equals(rk) && on.startsWith("java.util.")) {
            if (name.equals("size") && n == 0) {
                return size(recv);
            }
            if (name.equals("isEmpty") && n == 0) {
                return binop("==", intrinsic("len", List.of(recv), tInt("platform")), lit("0", tInt("platform")), tBool());
            }
            if (name.equals("get") && n == 1) {
                return index(recv, coerce(expr(as.get(0), ctx), tInt("32")), sub(rt, "elem"));
            }
        }
        if ("map".equals(rk) && on.startsWith("java.util.")) {
            Map<String, Object> elem = sub(rt, "elem");
            if (name.equals("size") && n == 0) {
                return size(recv);
            }
            if (name.equals("get") && n == 1) {
                if (!"ptr".equals(elem.get("k"))) {
                    throw unsupported("Map.get of a value that is not a reference: an absent key gives null, not a zero value");
                }
                return index(recv, coerce(expr(as.get(0), ctx), sub(rt, "key")), elem);
            }
            if (name.equals("getOrDefault") && n == 2) {
                Map<String, Object> d = coerce(expr(as.get(1), ctx), elem);
                if (!isZeroLit(d)) {
                    throw unsupported("getOrDefault with a default other than the zero value");
                }
                return index(recv, coerce(expr(as.get(0), ctx), sub(rt, "key")), elem);
            }
        }
        if (rt.equals(tString()) && on.equals("java.lang.String")) {
            if (name.equals("equals") && n == 1) {
                Expression arg = as.get(0);
                ClassNode at = inferred(arg);
                if (arg instanceof GStringExpression || at != null && at.getName().equals("groovy.lang.GString")) {
                    throw refuse(GROOVY, "String.equals of a GString is false: a GString is not a String, though it prints as one");
                }
                Map<String, Object> y = expr(arg, ctx);
                if (!type(y).equals(tString())) {
                    throw unsupported("String.equals of something other than a String");
                }
                return binop("==", recv, y, tBool());
            }
            if (name.equals("isEmpty") && n == 0) {
                return binop("==", recv, lit("\"\"", tString()), tBool());
            }
        }
        throw unsupported(owner.getNameWithoutPackage() + "." + name + " is outside the reader's library table");
    }

    /** A call of a static method by its bare name, as the compiler rewrites one inside a static method. */
    private Map<String, Object> staticCall(StaticMethodCallExpression sc, Ctx ctx) {
        MethodNode target = sc.getNodeMetaData(StaticTypesMarker.DIRECT_METHOD_CALL_TARGET);
        if (target == null && sc.getOwnerType().redirect().isPrimaryClassNode()) {
            List<MethodNode> ms = sc.getOwnerType().getMethods(sc.getMethod());
            target = ms.size() == 1 ? ms.get(0) : null;
        }
        if (target == null) {
            throw refuse(DYNAMIC, "call of " + sc.getMethod() + ", which the type checker does not resolve");
        }
        List<Expression> as = args(sc.getArguments());
        for (Expression a : as) {
            if (a instanceof ClosureExpression || a instanceof MethodPointerExpression) {
                throw refuse(CLOSURE, "a closure passed to " + sc.getMethod() + ": functions as values are outside cir/1");
            }
        }
        Fn fn = fns.get(target);
        if (fn == null || !fn.isStatic) {
            throw unsupported("call of " + target.getDeclaringClass().getNameWithoutPackage() + "." + sc.getMethod() + ", which is not read");
        }
        checkCallInForEach(fn, sc.getMethod(), ctx);
        Map<String, Object> c = Json.obj();
        c.put("k", "call");
        c.put("name", fn.name);
        c.put("args", argValues(as, fn.params, ctx));
        c.put("t", fn.resultType());
        return c;
    }

    private List<Object> argValues(List<Expression> as, List<Map<String, Object>> params, Ctx ctx) {
        if (as.size() != params.size()) {
            throw unsupported("a call with " + as.size() + " arguments of a function of " + params.size() + " parameters");
        }
        List<Object> out = new ArrayList<>();
        for (int i = 0; i < as.size(); i++) {
            out.add(coerce(value(as.get(i), params.get(i), ctx), params.get(i)));
        }
        return out;
    }

    /**
     * {@code String.format} as {@code sprintf}: a literal format whose only conversions are {@code
     * %s} of a String, {@code %d} of an integer and {@code %%}, one argument per conversion.
     */
    private Map<String, Object> format(List<Expression> as, Ctx ctx) {
        if (as.isEmpty() || !(as.get(0) instanceof ConstantExpression) || !(((ConstantExpression) as.get(0)).getValue() instanceof String)) {
            throw unsupported("String.format of a format that is not a literal: its conversions are unknown");
        }
        String f = (String) ((ConstantExpression) as.get(0)).getValue();
        List<Character> verbs = new ArrayList<>();
        for (int i = 0; i < f.length(); i++) {
            if (f.charAt(i) != '%') {
                continue;
            }
            char v = i + 1 < f.length() ? f.charAt(i + 1) : ' ';
            if (v != 's' && v != 'd' && v != '%') {
                throw unsupported("String.format conversion " + f.substring(i, Math.min(i + 2, f.length()))
                    + ": only %s, %d and %% print as cir/1's sprintf prints them");
            }
            if (v != '%') {
                verbs.add(v);
            }
            i++;
        }
        if (verbs.size() != as.size() - 1) {
            throw unsupported("String.format with " + verbs.size() + " conversions and " + (as.size() - 1) + " arguments");
        }
        List<Object> argv = new ArrayList<>();
        argv.add(expr(as.get(0), ctx));
        for (int i = 1; i < as.size(); i++) {
            Map<String, Object> x = expr(as.get(i), ctx);
            Map<String, Object> t = type(x);
            boolean fits = verbs.get(i - 1) == 's' ? t.equals(tString()) : isInt(t);
            if (!fits) {
                throw unsupported("String.format %" + verbs.get(i - 1) + " of a value of type " + describe(t)
                    + ": only %s of a String and %d of an integer print alike");
            }
            argv.add(x);
        }
        return intrinsic("sprintf", argv, tString());
    }

    /** {@code size()}: {@code len}, of the platform int, converted to {@code int32}, the type Java gives it. */
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

    private Map<String, Object> newObject(ConstructorCallExpression c, Ctx ctx) {
        if (c.isSpecialCall()) {
            throw unsupported("this(…) or super(…): constructor chaining is outside cir/1");
        }
        if (c.isUsingAnonymousInnerClass()) {
            throw unsupported("anonymous class is outside cir/1");
        }
        List<Expression> as = args(c.getArguments());
        for (Expression a : as) {
            if (a instanceof MapExpression) {
                throw refuse(GROOVY, "named-argument constructor: Groovy sets each property after construction");
            }
            if (a instanceof ClosureExpression) {
                throw refuse(CLOSURE, "a closure passed to a constructor: functions as values are outside cir/1");
            }
        }
        ClassNode t = c.getType();
        ClassNode s = structs.get(t.getName());
        if (s != null) {
            Fn fn = ctors.get(s);
            checkCallInForEach(fn, "new " + s.getNameWithoutPackage(), ctx);
            Map<String, Object> call = Json.obj();
            call.put("k", "call");
            call.put("name", fn.name);
            call.put("args", argValues(as, fn.params, ctx));
            call.put("t", fn.resultType());
            return call;
        }
        if (isNewCollection(c, FRESH)) {
            Map<String, Object> mk = Json.obj();
            mk.put("k", "make");
            mk.put("t", typeOf(t));
            return mk;
        }
        throw unsupported("new " + t.getNameWithoutPackage() + " is outside the reader's library table");
    }

    private Map<String, Object> binary(BinaryExpression b, Ctx ctx) {
        int op = b.getOperation().getType();
        switch (op) {
            case Types.EQUAL:
                throw unsupported("an assignment inside an expression");
            case Types.LEFT_SQUARE_BRACKET:
                throw refuse(GROOVY, "[] calls getAt, which counts negative indices from the end and gives null past it");
            case Types.KEYWORD_IN:
                throw refuse(GROOVY, "in calls isCase");
            case Types.COMPARE_TO:
                throw refuse(GROOVY, "<=> calls compareTo");
            case Types.POWER:
                throw refuse(GROOVY, "** is Groovy's power operator");
            case Types.FIND_REGEX:
            case Types.MATCH_REGEX:
                throw unsupported("regular expressions are outside cir/1");
            case Types.LEFT_SHIFT:
            case Types.RIGHT_SHIFT:
            case Types.RIGHT_SHIFT_UNSIGNED:
                throw unsupported("shift: the shift count's masking is outside cir/1");
            case Types.KEYWORD_INSTANCEOF:
                throw unsupported("instanceof: runtime types are outside cir/1");
            case Types.LOGICAL_AND:
            case Types.LOGICAL_OR: {
                Map<String, Object> x = expr(b.getLeftExpression(), ctx);
                Map<String, Object> y = expr(b.getRightExpression(), ctx);
                for (Map<String, Object> o : List.of(x, y)) {
                    if (!type(o).equals(tBool())) {
                        throw truth(type(o));
                    }
                }
                return binop(op == Types.LOGICAL_AND ? "&&" : "||", x, y, tBool());
            }
            case Types.COMPARE_EQUAL:
            case Types.COMPARE_NOT_EQUAL:
                return equality(b, op == Types.COMPARE_EQUAL ? "==" : "!=", ctx);
            case Types.COMPARE_LESS_THAN:
            case Types.COMPARE_LESS_THAN_EQUAL:
            case Types.COMPARE_GREATER_THAN:
            case Types.COMPARE_GREATER_THAN_EQUAL: {
                Map<String, Object> x = expr(b.getLeftExpression(), ctx);
                Map<String, Object> y = expr(b.getRightExpression(), ctx);
                String o = op == Types.COMPARE_LESS_THAN ? "<" : op == Types.COMPARE_LESS_THAN_EQUAL ? "<="
                    : op == Types.COMPARE_GREATER_THAN ? ">" : ">=";
                if (!isInt(type(x)) || !isInt(type(y))) {
                    throw unsupported(o + " on a value of type " + describe(isInt(type(x)) ? type(y) : type(x)) + ": Groovy calls compareTo");
                }
                List<Map<String, Object>> xy = unify(x, y);
                return binop(o, xy.get(0), xy.get(1), tBool());
            }
            case Types.PLUS:
                if (isStringType(inferred(b))) {
                    return concat(b, ctx);
                }
                return arithmetic(b, "+", ctx);
            case Types.MINUS:
                return arithmetic(b, "-", ctx);
            case Types.MULTIPLY:
                return arithmetic(b, "*", ctx);
            case Types.DIVIDE:
                return arithmetic(b, "/", ctx);
            case Types.MOD:
                return arithmetic(b, "%", ctx);
            case Types.BITWISE_AND:
                return arithmetic(b, "&", ctx);
            case Types.BITWISE_OR:
                return arithmetic(b, "|", ctx);
            case Types.BITWISE_XOR:
                return arithmetic(b, "^", ctx);
            default:
                throw unsupported("operator " + b.getOperation().getText() + " is outside cir/1");
        }
    }

    private Map<String, Object> arithmetic(BinaryExpression b, String op, Ctx ctx) {
        Map<String, Object> x = expr(b.getLeftExpression(), ctx);
        Map<String, Object> y = expr(b.getRightExpression(), ctx);
        if (op.equals("/") && isInt(type(x)) && isInt(type(y))) {
            throw refuse(GROOVY, "integer / integer is a BigDecimal division in Groovy, not a truncating one");
        }
        Map<String, Object> t = typeOfInferred(b);
        if (!isInt(t)) {
            throw unsupported(op + " on a value of type " + describe(t));
        }
        return binop(op, coerce(x, t), coerce(y, t), t);
    }

    /**
     * {@code ==} and {@code !=}: Groovy calls {@code equals}, which on Strings, integers, booleans and
     * enums is the value comparison cir/1 means; against {@code null} it is a nil check.
     */
    private Map<String, Object> equality(BinaryExpression b, String op, Ctx ctx) {
        Map<String, Object> x = expr(b.getLeftExpression(), ctx);
        Map<String, Object> y = expr(b.getRightExpression(), ctx);
        boolean againstNull = type(x).equals(NULL) || type(y).equals(NULL);
        if (!againstNull) {
            for (Map<String, Object> o : List.of(x, y)) {
                Map<String, Object> t = type(o);
                if (!t.equals(tString()) && !isInt(t) && !t.equals(tBool()) && enumName(t) == null) {
                    throw refuse(GROOVY, op + " on a value of type " + describe(t) + " calls equals, which cir/1 does not say of it");
                }
            }
        }
        if (type(x).equals(NULL)) {
            x = coerce(x, type(y));
        } else if (type(y).equals(NULL)) {
            y = coerce(y, type(x));
        } else {
            List<Map<String, Object>> xy = unify(x, y);
            x = xy.get(0);
            y = xy.get(1);
        }
        return binop(op, x, y, tBool());
    }

    /** Two operands of one type: an integer literal takes the other's type. */
    private List<Map<String, Object>> unify(Map<String, Object> x, Map<String, Object> y) {
        if (!type(x).equals(type(y)) && isIntLit(y)) {
            y = coerce(y, type(x));
        } else {
            x = coerce(x, type(y));
        }
        return List.of(x, y);
    }

    private static boolean isIntLit(Map<String, Object> e) {
        return "lit".equals(e.get("k")) && isInt(type(e));
    }

    /**
     * String concatenation: of Strings only, {@code +}; otherwise {@code sprintf} with {@code %s}
     * for a String or enum, {@code %d} for an integer and {@code %t} for a boolean, each printing
     * what Groovy's {@code toString} prints.
     */
    private Map<String, Object> concat(BinaryExpression b, Ctx ctx) {
        List<Expression> parts = new ArrayList<>();
        flatten(b, parts);
        List<Map<String, Object>> xs = new ArrayList<>();
        boolean allStrings = true;
        StringBuilder fmt = new StringBuilder();
        for (Expression part : parts) {
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
                throw unsupported("string concatenation of a value of type " + describe(t));
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

    private void flatten(Expression e, List<Expression> out) {
        if (e instanceof BinaryExpression && ((BinaryExpression) e).getOperation().getType() == Types.PLUS
            && isStringType(inferred(e))) {
            flatten(((BinaryExpression) e).getLeftExpression(), out);
            flatten(((BinaryExpression) e).getRightExpression(), out);
        } else {
            out.add(e);
        }
    }

    private static boolean isStringType(ClassNode c) {
        return c != null && (c.getName().equals("java.lang.String") || c.getName().equals("groovy.lang.GString"));
    }

    // ---- types ----

    private static final Map<String, Object> NULL = Map.of("k", "null");

    /** The type the checker infers for an expression, else its declared type. */
    private ClassNode inferred(Expression e) {
        return chooser.resolveType(e, ClassHelper.OBJECT_TYPE);
    }

    private Map<String, Object> typeOfInferred(Expression e) {
        ClassNode c = inferred(e);
        if (c == null || c.getName().equals("java.lang.Object")) {
            throw refuse(DYNAMIC, "no inferred type for " + e.getText() + ": Groovy decides at run time");
        }
        return typeOf(c);
    }

    /** The class an expression has by declaration: a variable's declared type, else the inferred one. */
    private ClassNode staticType(Expression e, Ctx ctx) {
        if (e instanceof VariableExpression) {
            Variable v = ((VariableExpression) e).getAccessedVariable();
            if (v instanceof PropertyNode) {
                return ((PropertyNode) v).getField().getType();
            }
            if (v instanceof FieldNode) {
                return ((FieldNode) v).getType();
            }
            if (v != null && !v.isDynamicTyped()) {
                return v.getOriginType();
            }
            if (v instanceof VariableExpression) {
                ClassNode c = ((VariableExpression) v).getNodeMetaData(StaticTypesMarker.DECLARATION_INFERRED_TYPE);
                if (c != null) {
                    return c;
                }
            }
        }
        if (e instanceof PropertyExpression) {
            Object n = named(e);
            if (n instanceof FieldNode) {
                return ((FieldNode) n).getType();
            }
        }
        ClassNode c = inferred(e);
        return c == null ? ClassHelper.OBJECT_TYPE : c;
    }

    /** The cir/1 type of a class: the JVM type table of {@link JvmTypes}, with the read classes. */
    private Map<String, Object> typeOf(ClassNode c) {
        try {
            return JvmTypes.map(jvmType(c), this::ownType);
        } catch (JvmTypes.Outside o) {
            throw refuse(TYPE, o.getMessage());
        }
    }

    /** A class as the JVM type table describes it. */
    private static JvmTypes.Type jvmType(ClassNode c) {
        if (c.isArray()) {
            return JvmTypes.Type.other(text(c));
        }
        if (ClassHelper.isPrimitiveType(c)) {
            switch (c.getName()) {
                case "int":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.INT, "int");
                case "long":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.LONG, "long");
                case "short":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.SHORT, "short");
                case "byte":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.BYTE, "byte");
                case "boolean":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.BOOLEAN, "boolean");
                case "char":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.CHAR, "char");
                case "float":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.FLOAT, "float");
                case "double":
                    return JvmTypes.Type.primitive(JvmTypes.Primitive.DOUBLE, "double");
                default:
                    return JvmTypes.Type.other(c.getName());
            }
        }
        List<JvmTypes.Type> args = new ArrayList<>();
        GenericsType[] gs = c.getGenericsTypes();
        if (gs != null) {
            for (GenericsType g : gs) {
                args.add(g.isPlaceholder() || g.isWildcard() ? JvmTypes.Type.other(g.getName()) : jvmType(g.getType()));
            }
        }
        return JvmTypes.Type.declared(c.getName(), args, text(c));
    }

    /** A class printed as javac prints a type: {@code java.util.Map<java.lang.String,java.lang.Integer>}. */
    private static String text(ClassNode c) {
        if (c.isArray()) {
            return text(c.getComponentType()) + "[]";
        }
        GenericsType[] gs = c.getGenericsTypes();
        if (gs == null || gs.length == 0 || ClassHelper.isPrimitiveType(c)) {
            return c.getName();
        }
        StringBuilder b = new StringBuilder(c.getName()).append('<');
        for (int i = 0; i < gs.length; i++) {
            b.append(i > 0 ? "," : "").append(gs[i].isPlaceholder() || gs[i].isWildcard() ? gs[i].getName() : text(gs[i].getType()));
        }
        return b.append('>').toString();
    }

    /** A read class: a struct as {@code ptr(named)}, an enum as {@code named}; else null, which the table refuses. */
    private Map<String, Object> ownType(JvmTypes.Type t) {
        ClassNode s = structs.get(t.className());
        if (s != null) {
            return tPtr(tNamed(s.getNameWithoutPackage()));
        }
        ClassNode e = enums.get(t.className());
        if (e != null) {
            return tNamed(e.getNameWithoutPackage());
        }
        return null;
    }

    private String enumName(Map<String, Object> t) {
        if (!"named".equals(t.get("k"))) {
            return null;
        }
        for (ClassNode e : enums.values()) {
            if (e.getNameWithoutPackage().equals(t.get("name"))) {
                return e.getNameWithoutPackage();
            }
        }
        return null;
    }

    private static boolean isInt(Map<String, Object> t) {
        return "int".equals(t.get("k"));
    }

    /** A cir/1 type as a reason names it: {@code int32}, {@code []string}, {@code map[string]int64}. */
    private static String describe(Map<String, Object> t) {
        if (t == null) {
            return "value";
        }
        switch (String.valueOf(t.get("k"))) {
            case "int":
                return (Boolean.TRUE.equals(t.get("signed")) ? "int" : "uint") + t.get("width");
            case "slice":
                return "[]" + describe(sub(t, "elem"));
            case "map":
                return "map[" + describe(sub(t, "key")) + "]" + describe(sub(t, "elem"));
            case "ptr":
                return "*" + describe(sub(t, "elem"));
            case "named":
                return String.valueOf(t.get("name"));
            default:
                return String.valueOf(t.get("k"));
        }
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
     * given a reference type; an integer literal typed as {@code want} when its value fits. Every
     * other conversion is outside the subset.
     */
    private Map<String, Object> coerce(Map<String, Object> e, Map<String, Object> want) {
        Map<String, Object> t = type(e);
        if (t.equals(want)) {
            return e;
        }
        if (t.equals(NULL)) {
            if (!"ptr".equals(want.get("k"))) {
                throw unsupported("null where a value of type " + describe(want) + " is expected");
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
        throw unsupported("a value of type " + describe(t) + " where a value of type " + describe(want) + " is expected: implicit conversions are outside cir/1");
    }

    /** A literal of a constant's value, typed as the constant. */
    private Map<String, Object> constLit(Object v, Map<String, Object> t) {
        Map<String, Object> l;
        if (v instanceof String) {
            l = lit(quote((String) v), tString());
        } else if (v instanceof Boolean) {
            l = lit(String.valueOf(v), tBool());
        } else if (v instanceof Integer || v instanceof Short || v instanceof Byte) {
            l = lit(String.valueOf(v), tInt("32"));
        } else if (v instanceof Long) {
            l = lit(String.valueOf(v), tInt("64"));
        } else {
            throw refuse(TYPE, JvmTypes.outside(v == null ? "null" : v.getClass().getName()));
        }
        return coerce(l, t);
    }

    /** Refuse a statement whose output still holds an untyped null. */
    private static void checkTyped(Object o) {
        if (o instanceof Map) {
            Map<?, ?> m = (Map<?, ?>) o;
            if (NULL.equals(m.get("t"))) {
                throw unsupported("null whose type the context does not give");
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

    private static Map<String, Object> kind(String k) {
        Map<String, Object> t = Json.obj();
        t.put("k", k);
        return t;
    }

    private static Map<String, Object> tInt(String width) {
        Map<String, Object> t = kind("int");
        t.put("width", width);
        t.put("signed", true);
        return t;
    }

    private static Map<String, Object> tBool() {
        return kind("bool");
    }

    private static Map<String, Object> tString() {
        return kind("string");
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
                throw unsupported("a string with an unpaired surrogate");
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

    // ---- positions, source text, type-checking errors and refusals ----

    private static String pos(Unit u, ASTNode n, int fallback) {
        return u.file + ":" + (n.getLineNumber() > 0 ? n.getLineNumber() : fallback);
    }

    private static String pos(Ctx ctx, ASTNode n) {
        return pos(ctx.unit, n, ctx.fallbackLine);
    }

    /** A declaration's position: where it starts after its annotations. */
    private static String declPos(Unit u, AnnotatedNode n) {
        int line = n.getLineNumber();
        for (AnnotationNode a : n.getAnnotations()) {
            if (a.getLastLineNumber() < line || a.getLastLineNumber() < 1) {
                continue;
            }
            int l = a.getLastLineNumber();
            int c = a.getLastColumnNumber() - 1;
            while (l <= u.lines.length) {
                String text = u.lines[l - 1];
                while (c < text.length() && Character.isWhitespace(text.charAt(c))) {
                    c++;
                }
                if (c < text.length()) {
                    break;
                }
                l++;
                c = 0;
            }
            line = Math.max(line, l);
        }
        return u.file + ":" + line;
    }

    /** The source text of a node, from its first to its last column. */
    private static String source(Unit u, ASTNode n) {
        int l1 = n.getLineNumber();
        int c1 = n.getColumnNumber();
        int l2 = n.getLastLineNumber();
        int c2 = n.getLastColumnNumber();
        if (l1 < 1 || l2 < l1 || l2 > u.lines.length || c1 < 1 || c2 < 1) {
            return n.getText();
        }
        StringBuilder b = new StringBuilder();
        for (int l = l1; l <= l2; l++) {
            String text = u.lines[l - 1];
            int from = l == l1 ? Math.min(c1 - 1, text.length()) : 0;
            int to = l == l2 ? Math.min(c2 - 1, text.length()) : text.length();
            b.append(text, from, Math.max(from, to));
            if (l < l2) {
                b.append('\n');
            }
        }
        return b.toString();
    }

    private static String firstLine(String s) {
        int i = s.indexOf('\n');
        return i < 0 ? s : s.substring(0, i) + " …";
    }

    private static boolean within(Err e, ASTNode n) {
        if (n.getLineNumber() < 1) {
            return false;
        }
        int sl = n.getLineNumber();
        int sc = n.getColumnNumber();
        int el = n.getLastLineNumber();
        int ec = n.getLastColumnNumber();
        boolean afterStart = e.line > sl || e.line == sl && e.col >= sc;
        boolean beforeEnd = e.line < el || e.line == el && e.col <= ec;
        return afterStart && beforeEnd;
    }

    /** The first type-checking error inside a node no nested statement answered, or null. */
    private Err firstError(Unit u, ASTNode n) {
        for (Err e : errs) {
            if (!e.consumed && e.unit == u && within(e, n)) {
                return e;
            }
        }
        return null;
    }

    /** Mark every type-checking error inside a node answered: the node is refused as a whole. */
    private void consume(Unit u, ASTNode n) {
        for (Err e : errs) {
            if (e.unit == u && within(e, n)) {
                e.consumed = true;
            }
        }
    }

    private void addRefusal(String pos, String what, String reason) {
        Map<String, Object> r = Json.obj();
        r.put("pos", pos);
        r.put("what", what);
        r.put("reason", reason);
        refusals.add(r);
    }

    private void truncate(int mark) {
        while (refusals.size() > mark) {
            refusals.remove(refusals.size() - 1);
        }
    }

    private void refuseDecl(Unit u, AnnotatedNode n, String what, String reason) {
        addRefusal(declPos(u, n), what, reason);
    }

    private Map<String, Object> foreign(Unit u, ASTNode n, String reason, Ctx ctx) {
        String src = source(u, n);
        String pos = pos(u, n, ctx.fallbackLine);
        addRefusal(pos, firstLine(src), reason);
        Map<String, Object> st = Json.obj();
        st.put("k", "foreign");
        st.put("lang", "groovy");
        st.put("src", src);
        st.put("reason", reason);
        st.put("pos", pos);
        return st;
    }
}

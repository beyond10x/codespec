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
import com.sun.source.tree.ImportTree;
import com.sun.source.tree.LabeledStatementTree;
import com.sun.source.tree.LambdaExpressionTree;
import com.sun.source.tree.LiteralTree;
import com.sun.source.tree.MemberReferenceTree;
import com.sun.source.tree.NewArrayTree;
import com.sun.source.tree.ArrayAccessTree;
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
import java.math.BigInteger;
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
import java.util.TreeMap;
import java.util.stream.Stream;
import javax.lang.model.element.AnnotationMirror;
import javax.lang.model.element.Element;
import javax.lang.model.element.ElementKind;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.Modifier;
import javax.lang.model.element.PackageElement;
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
 *       local holding it, refused through a receiver that is not a name, whose call Java makes
 *       before the value. Every other read dereferences the pointer. A read Java would print
 *       (concatenation), unbox or compare (an operator, {@code +=}, {@code ++}) or hand on (a
 *       return, an argument, a local's initializer, an assignment), and a collection write to
 *       such a field, are refused at the use, since the written program stops at the nil pointer
 *       where Java goes on with null. A read as a receiver, where Java throws on null, assumes an
 *       object is there (README.md, Assumptions).
 *   <li>{@code ==} between two objects other than enum values is refused (identity); against
 *       {@code null} it is a nil check. A static member reached through an expression is refused,
 *       since Java evaluates and drops that expression.
 * </ul>
 *
 * <p>In a package read (not a project's, whose modules stay as they were) it also maps, each
 * listed with its limits in README.md: shifts with the count masked as Java masks it; a labeled
 * {@code break} or {@code continue} of the innermost loop; {@code double} and {@code float}; a
 * mutable static field of a primitive or {@code String} as a module-level {@code var};
 * {@code Object} as {@code any}; an interface with several implementations as a cir/1 interface;
 * an unbounded static generic method; the primitive functional interfaces of {@code
 * java.util.function} as function types, with lambdas, method references and calls of them;
 * variable arity methods and arrays as slices; an exception leaf whose constructor formats its
 * parameters as {@code errorf}; {@code System.out.println} of an integer, a boolean or a
 * String. A sealed interface whose permitted classes are records without members is a union,
 * refused at its line while {@link #READ_UNIONS} is off.
 *
 * <p>A project directory (one with {@code pom.xml}, {@code build.gradle} or {@code
 * build.gradle.kts}) is read with {@link #readProject}: every package under {@code src/main/java}
 * in one javac analysis, one module per package, against the jars of {@code
 * CODESPEC_JAVA_CLASSPATH}. In a project read a type or call the table does not map becomes an
 * extern: a call into a jar is an extern with its declared signature, a class of another package
 * of the project an extern of that package. Quarkus 3 maps to the additive {@code entries}, {@code
 * beans} and {@code injects} of the module: {@code @Path} with a verb is an {@code http} entry
 * point, {@code @Scheduled} and {@code @Incoming} entry points, {@code @ApplicationScoped},
 * {@code @Singleton}, {@code @RequestScoped} and {@code @Dependent} a bean, {@code @Transactional} a
 * mark in {@code transactional}, {@code @Produces} and {@code @Consumes} the media types of an http entry, {@code @Inject} an
 * injection resolved to the one bean or to an extern. A {@code @RegisterRestClient} interface is
 * externs, and Panache, {@code EntityManager} and JDBC calls are externs ({@link
 * #effectsSummary} gives them {@code net} and {@code db}).
 *
 * <p>A project read (not a package's, which refuses them as before) also maps, each listed with its
 * refused forms in README.md: a second constructor as {@code New<Class>2}, the n-th as {@code
 * New<Class><n>}; a static nested class as a struct of its simple name; a {@code static final}
 * field set by a literal or by a jar's static method with constant arguments as a module-level
 * variable, {@code C.class.getName()} as the literal of the class's binary name; the {@code id} a
 * class extending {@code PanacheEntity} inherits, as the struct's first field; and a static Panache
 * method called on an entity as an extern call with the entity as its type argument. It records the
 * JPA annotations {@code @Entity}, {@code @Cacheable}, {@code @Id}, {@code @GeneratedValue} and
 * {@code @Column} on a struct or its instance field in {@code persistence}, each with what it marks
 * and its arguments on one line. A {@code @Provider} class implementing {@code ExceptionMapper} of
 * a jar's exception is an {@code exception-mapper} entry naming its {@code toResponse}; any other
 * is refused at its line. A {@code throw new X(…)} of a jar's unchecked exception in a method an
 * entry names is a {@code raise}, listed in the function's {@code raises} ({@link #raise}); a call
 * of such a method is refused. A path parameter only the framework passes is printed as it is
 * ({@link #pathParameter}). Refused too: a field of a class of another package, whose object is an opaque extern here, a
 * call of another package's method or constructor that package's module does not declare in a form
 * this module can call ({@link #declaredFn}; the extern takes that module's signature), and an
 * access of a managed class's field that Quarkus turns into a call of code ({@link
 * #managedAccessor}).
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
    private final Analysis analysis;
    /** Whether the module is read as part of a project: its jars and other packages are externs. */
    private final boolean project;
    /** The package this module reads, in project mode. */
    private String pkgName = "";

    /** Every extern the module may call, by path, receiver type name and name. */
    private final Map<String, Map<String, Object>> externs = new TreeMap<>();
    private final List<Object> outEntries = new ArrayList<>();
    private final List<Object> outBeans = new ArrayList<>();
    private final List<Object> outInjects = new ArrayList<>();
    private final List<String> outTransactional = new ArrayList<>();
    private final List<Object> outPersistence = new ArrayList<>();
    /**
     * The JPA annotations a project read records in {@code persistence} instead of refusing, on a
     * class and on its instance fields; anywhere else they stay refusals.
     */
    private static final Set<String> PERSISTENCE = Set.of("jakarta.persistence.Entity", "jakarta.persistence.Cacheable",
        "jakarta.persistence.Id", "jakarta.persistence.GeneratedValue", "jakarta.persistence.Column");

    private static final String REST_CLIENT = "org.eclipse.microprofile.rest.client.inject.RegisterRestClient";
    private static final String REST_CLIENT_QUALIFIER = "org.eclipse.microprofile.rest.client.inject.RestClient";
    private static final String INJECT = "jakarta.inject.Inject";
    private static final String PATH = "jakarta.ws.rs.Path";
    private static final String TRANSACTIONAL = "jakarta.transaction.Transactional";
    private static final String PRODUCES = "jakarta.ws.rs.Produces";
    private static final String CONSUMES = "jakarta.ws.rs.Consumes";
    private static final String SCHEDULED = "io.quarkus.scheduler.Scheduled";
    private static final String INCOMING = "org.eclipse.microprofile.reactive.messaging.Incoming";
    private static final List<String> HTTP_METHODS = List.of("GET", "POST", "PUT", "DELETE", "PATCH");
    private static final String EXCEPTION_MAPPER = "jakarta.ws.rs.ext.ExceptionMapper";
    /** The registration of an {@link #EXCEPTION_MAPPER}: on one, an {@code exception-mapper} entry, elsewhere a refusal. */
    private static final String PROVIDER = "jakarta.ws.rs.ext.Provider";
    private static final String PATH_PARAM = "jakarta.ws.rs.PathParam";
    /** The Panache superclass whose one field, {@code id}, a project read gives every entity extending it. */
    private static final String PANACHE_ENTITY = "io.quarkus.hibernate.orm.panache.PanacheEntity";
    private static final List<String> PANACHE_ENTITY_BASES = List.of(
        "io.quarkus.hibernate.orm.panache.PanacheEntityBase", "io.quarkus.hibernate.reactive.panache.PanacheEntityBase");
    /** The JPA annotations of a class Quarkus manages: it rewrites accesses of its public fields into accessor calls. */
    private static final List<String> MANAGED = List.of(
        "jakarta.persistence.Entity", "jakarta.persistence.MappedSuperclass", "jakarta.persistence.Embeddable");
    private static final List<String> SCOPES = List.of(
        "jakarta.enterprise.context.ApplicationScoped", "jakarta.inject.Singleton",
        "jakarta.enterprise.context.RequestScoped", "jakarta.enterprise.context.Dependent");

    /** The annotations a project read maps instead of refusing, by where they may stand. */
    private static final Set<String> QUARKUS_ANNOTATIONS = new HashSet<>(List.of(
        REST_CLIENT, REST_CLIENT_QUALIFIER, INJECT, PATH, SCHEDULED, INCOMING, TRANSACTIONAL, PRODUCES, CONSUMES,
        "jakarta.ws.rs.GET", "jakarta.ws.rs.POST", "jakarta.ws.rs.PUT", "jakarta.ws.rs.DELETE",
        "jakarta.ws.rs.PATCH", "jakarta.enterprise.context.ApplicationScoped", "jakarta.inject.Singleton",
        "jakarta.enterprise.context.RequestScoped", "jakarta.enterprise.context.Dependent"));

    /**
     * The calls whose effect is {@code db}, as path and a comma-separated list of names: a function
     * is its name, a method {@code Type.name}. Panache entities and repositories, the entity manager,
     * its queries and JDBC.
     */
    private static final String[][] DB_CALLS = {
        {"io.quarkus.hibernate.orm.panache.PanacheEntityBase",
            "PanacheEntityBase.delete,PanacheEntityBase.flush,PanacheEntityBase.isPersistent,PanacheEntityBase.persist,"
                + "PanacheEntityBase.persistAndFlush,count,delete,deleteAll,deleteById,find,findAll,findById,"
                + "findByIdOptional,flush,getEntityManager,list,listAll,persist,stream,streamAll,update"},
        {"io.quarkus.hibernate.orm.panache.PanacheRepositoryBase",
            "PanacheRepositoryBase.count,PanacheRepositoryBase.delete,PanacheRepositoryBase.deleteAll,"
                + "PanacheRepositoryBase.deleteById,PanacheRepositoryBase.find,PanacheRepositoryBase.findAll,"
                + "PanacheRepositoryBase.findById,PanacheRepositoryBase.findByIdOptional,PanacheRepositoryBase.flush,"
                + "PanacheRepositoryBase.getEntityManager,PanacheRepositoryBase.isPersistent,PanacheRepositoryBase.list,"
                + "PanacheRepositoryBase.listAll,PanacheRepositoryBase.persist,PanacheRepositoryBase.persistAndFlush,"
                + "PanacheRepositoryBase.stream,PanacheRepositoryBase.streamAll,PanacheRepositoryBase.update"},
        {"io.quarkus.hibernate.reactive.panache.PanacheEntityBase",
            "PanacheEntityBase.delete,PanacheEntityBase.flush,PanacheEntityBase.isPersistent,PanacheEntityBase.persist,"
                + "PanacheEntityBase.persistAndFlush,count,delete,deleteAll,deleteById,find,findAll,findById,"
                + "flush,list,listAll,persist,stream,streamAll,update"},
        {"io.quarkus.hibernate.reactive.panache.PanacheRepositoryBase",
            "PanacheRepositoryBase.count,PanacheRepositoryBase.delete,PanacheRepositoryBase.deleteAll,"
                + "PanacheRepositoryBase.deleteById,PanacheRepositoryBase.find,PanacheRepositoryBase.findAll,"
                + "PanacheRepositoryBase.findById,PanacheRepositoryBase.flush,PanacheRepositoryBase.isPersistent,"
                + "PanacheRepositoryBase.list,PanacheRepositoryBase.listAll,PanacheRepositoryBase.persist,"
                + "PanacheRepositoryBase.persistAndFlush,PanacheRepositoryBase.stream,PanacheRepositoryBase.streamAll,"
                + "PanacheRepositoryBase.update"},
        {"jakarta.persistence.EntityManager",
            "EntityManager.clear,EntityManager.createNamedQuery,EntityManager.createNativeQuery,"
                + "EntityManager.createQuery,EntityManager.detach,EntityManager.find,EntityManager.flush,"
                + "EntityManager.getReference,EntityManager.lock,EntityManager.merge,EntityManager.persist,"
                + "EntityManager.refresh,EntityManager.remove"},
        {"jakarta.persistence.Query",
            "Query.executeUpdate,Query.getResultList,Query.getResultStream,Query.getSingleResult"},
        {"jakarta.persistence.TypedQuery",
            "TypedQuery.executeUpdate,TypedQuery.getResultList,TypedQuery.getResultStream,TypedQuery.getSingleResult"},
        {"java.sql.Connection",
            "Connection.commit,Connection.createStatement,Connection.prepareCall,Connection.prepareStatement,"
                + "Connection.rollback"},
        {"java.sql.DriverManager", "getConnection"},
        {"java.sql.PreparedStatement",
            "PreparedStatement.execute,PreparedStatement.executeBatch,PreparedStatement.executeQuery,"
                + "PreparedStatement.executeUpdate"},
        {"java.sql.ResultSet",
            "ResultSet.getBoolean,ResultSet.getInt,ResultSet.getLong,ResultSet.getObject,ResultSet.getString,"
                + "ResultSet.next"},
        {"java.sql.Statement",
            "Statement.execute,Statement.executeBatch,Statement.executeQuery,Statement.executeUpdate"},
        {"javax.sql.DataSource", "DataSource.getConnection"},
    };

    private final Map<CompilationUnitTree, String> sources = new HashMap<>();
    /** Every compilation unit, in file order. */
    private final List<CompilationUnitTree> units = new ArrayList<>();

    /**
     * Top-level type declarations, in file then source order; in a project read each followed by the
     * static classes nested in it that are read ({@link #nested}).
     */
    private final List<TreePath> decls = new ArrayList<>();
    /** The static nested classes of a project read that are read as structs of their own. */
    private final Set<Tree> nested = new HashSet<>();
    /** Why each class nested in a top-level class of a project read is not read. */
    private final Map<Tree, String> nestedWhy = new HashMap<>();
    /** The structs the other packages of a project read, which this module names as their modules do. */
    private final Set<TypeElement> otherStructs = new HashSet<>();
    /** Whether {@link #declared} ran {@link #signatures} on this package's classification. */
    private boolean signed;
    private final Set<TypeElement> structs = new HashSet<>();
    private final Set<TypeElement> enums = new HashSet<>();
    private final Set<TypeElement> interfaces = new HashSet<>();
    private final Map<TypeElement, TypeElement> implOf = new HashMap<>();
    /** The interfaces of a package read with more than one implementation, cir/1 interfaces, with those classes. */
    private final Map<TypeElement, List<TypeElement>> impls = new HashMap<>();
    /** The methods of each interface of {@link #impls}, as cir/1 interface methods sorted by name. */
    private final Map<TypeElement, List<Map<String, Object>>> ifaceMethods = new HashMap<>();
    /**
     * The sealed interfaces of a package read whose permitted classes are records of the package
     * without members of their own, cir/1 unions, with those records in {@code permits} order.
     */
    private final Map<TypeElement, List<TypeElement>> unions = new HashMap<>();
    /**
     * Whether a sealed interface of records is emitted as a union. Off: the reader's check writes
     * every read as Go, and the Go writer refuses a union, so a union read cannot be checked; the
     * interface is refused at its line instead, its records with it.
     */
    static final boolean READ_UNIONS = false;
    /** The generic static methods the module reads. */
    private final Set<ExecutableElement> genericMethods = new HashSet<>();
    /** The static fields that are module-level variables, with their names. */
    private final Map<VariableElement, String> vars = new HashMap<>();
    /**
     * The exception leaves whose one constructor passes {@code String.format("<literal>", p1, …, pn)}
     * of its parameters, in order, to {@code super}: a throw is that format, {@code errorf}.
     */
    private final Map<TypeElement, String> formattedLeaves = new HashMap<>();
    private final List<Object> outInterfaces = new ArrayList<>();
    private final List<Object> outVars = new ArrayList<>();
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
        /** A variable arity method: its last parameter is the slice of the arguments it takes. */
        boolean variadic;
        /** The type parameters of a generic static method, each over {@code any}. */
        List<String> tparams = new ArrayList<>();
        TreePath path;
        /** A lambda's body: Java compiles it to a method of its own, neither a constructor nor an accessor. */
        boolean inLambda;

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
        /** The label of each enclosing loop, innermost first; {@code ""} for a loop without one. */
        final Deque<String> loopLabels = new ArrayDeque<>();
        /** The label of the labeled statement being read, until the loop it labels takes it. */
        String nextLabel = "";
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

    /**
     * What every module of one read shares: javac's analysis of all the files, and, for a project, the
     * classes it declares and its beans.
     */
    private static final class Analysis {
        final JavacTask task;
        final List<CompilationUnitTree> units;
        /** Qualified names of the top-level classes the project's own sources declare. */
        final Set<String> projectClasses = new HashSet<>();
        /** The beans of the project: a class with a scope annotation, in source order. */
        final List<TypeElement> beans = new ArrayList<>();
        /**
         * The compilation units of each package and its classification, made once: what another
         * package's module reads of its classes ({@link #otherPackages}).
         */
        final Map<String, List<CompilationUnitTree>> packages = new TreeMap<>();
        final Map<String, Reader> classified = new HashMap<>();
        /** {@link #mayRaise} of each method asked about, once for the read. */
        final Map<ExecutableElement, Boolean> mayRaise = new HashMap<>();
        /** The methods some source of the project calls or references, made once ({@link #calledInProject}). */
        Set<Element> called;

        Analysis(JavacTask task, List<CompilationUnitTree> units) {
            this.task = task;
            this.units = units;
        }
    }

    private Reader(Analysis analysis, boolean project) {
        this.trees = Trees.instance(analysis.task);
        this.types = analysis.task.getTypes();
        this.elements = analysis.task.getElements();
        this.positions = trees.getSourcePositions();
        this.analysis = analysis;
        this.project = project;
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
        Analysis a = analyse(dir, files, List.of());
        a.units.sort(Comparator.comparing(cu -> fileName(cu)));
        return new Reader(a, false).module(a.units);
    }

    /**
     * Whether {@code dir} is a project directory: it holds {@code pom.xml}, {@code build.gradle} or
     * {@code build.gradle.kts}.
     */
    public static boolean isProject(Path dir) {
        return Files.isRegularFile(dir.resolve("pom.xml")) || Files.isRegularFile(dir.resolve("build.gradle"))
            || Files.isRegularFile(dir.resolve("build.gradle.kts"));
    }

    /** The jars of a class path of the form {@code CODESPEC_JAVA_CLASSPATH} holds: paths separated by {@code :}. */
    public static List<Path> classpath(String value) {
        List<Path> out = new ArrayList<>();
        for (String s : value.split(":", -1)) {
            if (!s.isEmpty()) {
                out.add(Path.of(s));
            }
        }
        return out;
    }

    /** The {@code .java} files under {@code src/main/java}, sorted by path. */
    private static List<Path> projectFiles(Path dir) throws Failure {
        Path src = dir.resolve("src/main/java");
        if (!Files.isDirectory(src)) {
            throw new Failure(dir + ": no src/main/java");
        }
        try (Stream<Path> s = Files.walk(src)) {
            List<Path> files = s.filter(p -> Files.isRegularFile(p) && p.getFileName().toString().endsWith(".java")
                && !p.getFileName().toString().equals("module-info.java"))
                .sorted(Comparator.comparing(p -> src.relativize(p).toString()))
                .toList();
            if (files.isEmpty()) {
                throw new Failure(src + ": no .java files");
            }
            return files;
        } catch (IOException e) {
            throw new Failure(dir + ": " + e.getMessage());
        }
    }

    /** The packages of a project, in byte order: those of the files under {@code src/main/java}. */
    public static List<String> packages(Path dir) throws Failure {
        Path src = dir.resolve("src/main/java");
        Set<String> out = new java.util.TreeSet<>();
        for (Path f : projectFiles(dir)) {
            Path parent = src.relativize(f).getParent();
            out.add(parent == null ? "" : parent.toString().replace('/', '.'));
        }
        return new ArrayList<>(out);
    }

    /**
     * Read a project: every package under {@code src/main/java} in one javac analysis with {@code
     * classpath} on the class path, one module per package, by package name in byte order. A class
     * of another package or of a jar becomes an extern; the classes of the jars are not read.
     */
    public static Map<String, Map<String, Object>> readProject(Path dir, List<Path> classpath) throws Failure {
        Analysis a = projectAnalysis(dir, classpath);
        Map<String, List<CompilationUnitTree>> byPackage = new TreeMap<>();
        for (CompilationUnitTree cu : a.units) {
            byPackage.computeIfAbsent(cu.getPackageName() == null ? "" : cu.getPackageName().toString(), k -> new ArrayList<>()).add(cu);
        }
        a.packages.putAll(byPackage);
        Map<String, Map<String, Object>> out = new TreeMap<>();
        for (Map.Entry<String, List<CompilationUnitTree>> e : byPackage.entrySet()) {
            out.put(e.getKey(), new Reader(a, true).module(e.getValue()));
        }
        return out;
    }

    private static Analysis projectAnalysis(Path dir, List<Path> classpath) throws Failure {
        Analysis a = analyse(dir, projectFiles(dir), classpath);
        a.units.sort(Comparator.comparing((CompilationUnitTree cu) -> cu.getPackageName() == null ? "" : cu.getPackageName().toString())
            .thenComparing(cu -> fileName(cu)));
        Reader r = new Reader(a, true);
        for (CompilationUnitTree cu : a.units) {
            for (Tree t : cu.getTypeDecls()) {
                if (t instanceof ClassTree) {
                    TypeElement te = r.element(new TreePath(new TreePath(cu), t));
                    a.projectClasses.add(te.getQualifiedName().toString());
                }
            }
        }
        for (CompilationUnitTree cu : a.units) {
            for (Tree t : cu.getTypeDecls()) {
                if (t instanceof ClassTree) {
                    TreePath tp = new TreePath(new TreePath(cu), t);
                    List<TypeElement> classes = new ArrayList<>(List.of(r.element(tp)));
                    for (Tree m : ((ClassTree) t).getMembers()) {
                        if (m instanceof ClassTree) {
                            classes.add(r.element(new TreePath(tp, m)));
                        }
                    }
                    for (TypeElement te : classes) {
                        if (te.getKind() == ElementKind.CLASS && !te.getModifiers().contains(Modifier.ABSTRACT) && r.scope(te) != null
                            && (te.getNestingKind() == javax.lang.model.element.NestingKind.TOP_LEVEL || te.getModifiers().contains(Modifier.STATIC))) {
                            a.beans.add(te);
                        }
                    }
                }
            }
        }
        return a;
    }

    /**
     * The codespec-effects/1 summary of what a project's jars do outside the code: the calls of its
     * REST clients are {@code net}; those of Panache, the entity manager and JDBC are {@code db}.
     */
    public static String effectsSummary(Path dir, List<Path> classpath) throws Failure {
        Analysis a = projectAnalysis(dir, classpath);
        Reader r = new Reader(a, true);
        Map<String, List<String>> entries = new TreeMap<>();
        for (String[] db : DB_CALLS) {
            for (String name : db[1].split(",")) {
                entries.put(db[0] + "\u0000" + name, List.of("db"));
            }
        }
        for (CompilationUnitTree cu : a.units) {
            for (Tree t : cu.getTypeDecls()) {
                if (!(t instanceof ClassTree)) {
                    continue;
                }
                TypeElement te = r.element(new TreePath(new TreePath(cu), t));
                if (te.getKind() != ElementKind.INTERFACE || annotation(te, REST_CLIENT) == null) {
                    continue;
                }
                for (Element m : te.getEnclosedElements()) {
                    if (m.getKind() == ElementKind.METHOD && !m.getModifiers().contains(Modifier.STATIC)) {
                        entries.put(te.getQualifiedName() + "\u0000" + te.getSimpleName() + "." + m.getSimpleName(), List.of("net"));
                    }
                }
            }
        }
        Map<String, Object> doc = Json.obj();
        doc.put("format", "codespec-effects/1");
        doc.put("language", "java");
        List<Object> list = new ArrayList<>();
        for (Map.Entry<String, List<String>> e : entries.entrySet()) {
            int i = e.getKey().indexOf('\u0000');
            Map<String, Object> x = Json.obj();
            x.put("path", e.getKey().substring(0, i));
            x.put("name", e.getKey().substring(i + 1));
            x.put("effects", new ArrayList<Object>(e.getValue()));
            list.add(x);
        }
        doc.put("externs", list);
        return Json.write(doc);
    }

    /** Compile {@code files} against {@code classpath} and attribute them; a compile error is a failure. */
    private static Analysis analyse(Path dir, List<Path> files, List<Path> classpath) throws Failure {
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
            fm.setLocation(StandardLocation.CLASS_PATH, classpath.stream().map(Path::toFile).toList());
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
            if (classpath.isEmpty() && isProject(dir)) {
                errors.add("a project is read against its dependency jars: " + CLASSPATH_VARIABLE
                    + " names them, separated by ':'");
            }
            throw new Failure(String.join("\n", errors));
        }
        return new Analysis(task, units);
    }

    /** The environment variable that names a project's dependency jars. */
    public static final String CLASSPATH_VARIABLE = "CODESPEC_JAVA_CLASSPATH";

    private static String fileName(CompilationUnitTree cu) {
        return Path.of(cu.getSourceFile().getName()).getFileName().toString();
    }

    // ---- the module ----

    private Map<String, Object> module(List<CompilationUnitTree> units) throws Failure {
        prepare(units);
        String pkg = pkgName;
        if (project) {
            otherPackages();
        }
        signatures();
        for (TreePath p : decls) {
            emit(p);
        }
        pruneExterns();
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
        if (!outVars.isEmpty()) {
            doc.put("vars", outVars);
        }
        doc.put("funcs", outFuncs);
        if (!externs.isEmpty()) {
            doc.put("externs", new ArrayList<Object>(externs.values()));
        }
        doc.put("layout", layout);
        if (!refusals.isEmpty()) {
            doc.put("refusals", new ArrayList<Object>(refusals));
        }
        if (project) {
            doc.put("path", pkg);
        }
        if (!outInterfaces.isEmpty()) {
            doc.put("interfaces", outInterfaces);
        }
        if (!outEntries.isEmpty()) {
            doc.put("entries", outEntries);
        }
        if (!outBeans.isEmpty()) {
            doc.put("beans", outBeans);
        }
        if (!outInjects.isEmpty()) {
            doc.put("injects", outInjects);
        }
        if (!outTransactional.isEmpty()) {
            doc.put("transactional", new ArrayList<Object>(outTransactional));
        }
        if (!outPersistence.isEmpty()) {
            doc.put("persistence", outPersistence);
        }
        return doc;
    }

    /**
     * What the other packages of the project read, which this module's names of their classes
     * follow: the structs they make.
     */
    private void otherPackages() throws Failure {
        for (String pkg : analysis.packages.keySet()) {
            if (!pkg.equals(pkgName)) {
                otherStructs.addAll(declarer(pkg).structs);
            }
        }
    }

    /**
     * The classification of package {@code pkg} of the project, made once and shared: its structs,
     * and, once {@link #declared} asks, the functions its module declares.
     */
    private Reader declarer(String pkg) throws Failure {
        Reader other = analysis.classified.get(pkg);
        if (other == null) {
            other = new Reader(analysis, true);
            other.prepare(analysis.packages.get(pkg));
            analysis.classified.put(pkg, other);
        }
        return other;
    }

    /**
     * Whether the module of another package of the project declares what {@code e} names there: the
     * function of a method or constructor, the struct of a class, the interface of an interface. A
     * call or type of a class of another package names only what that module holds.
     */
    private boolean declared(Element e) {
        TypeElement te = e instanceof TypeElement ? (TypeElement) e : (TypeElement) e.getEnclosingElement();
        Reader other;
        try {
            other = declarer(packageOf(te));
        } catch (Failure f) {
            return false;
        }
        if (e instanceof TypeElement) {
            return other.structs.contains(te) || other.interfaces.contains(te) || other.enums.contains(te);
        }
        if (te.getKind() == ElementKind.INTERFACE) {
            return other.interfaces.contains(te);
        }
        return declaredFn(e) != null;
    }

    /**
     * The function the module of another package of the project declares for the method or
     * constructor {@code e}, or null when it declares none: its signature, not the Java one, is what
     * a call of it from this module must use. The default constructor javac adds is the {@code
     * New<Class>} that module makes up; a class declaring a constructor has no default one.
     */
    private Fn declaredFn(Element e) {
        TypeElement te = (TypeElement) e.getEnclosingElement();
        Reader other;
        try {
            other = declarer(packageOf(te));
        } catch (Failure f) {
            return null;
        }
        if (!other.signed) {
            other.signed = true;
            other.signatures();
        }
        if (other.fns.containsKey(e)) {
            return other.fns.get(e);
        }
        return e.getKind() == ElementKind.CONSTRUCTOR && generated(e) ? other.ctors.get(te) : null;
    }

    /**
     * Why a call of {@code fn}, a function another package's module declares, is not read here, or
     * null: its signature as that module declares it is what the extern must say, and this module has
     * no form for a call of one that returns an error (a checked exception), an Optional's {@code (T,
     * bool)}, or that is generic or variable arity.
     */
    private static String foreignCallShape(Fn fn) {
        if (fn.throwsErr || fn.foreignThrows != null) {
            return "it throws a checked exception, which its module declares as an error result this call would have to"
                + " handle";
        }
        if (fn.optional) {
            return "it returns an Optional, which its module declares as a value and a bool";
        }
        if (!fn.tparams.isEmpty() || fn.variadic) {
            return "its module declares it generic or of variable arity";
        }
        return null;
    }

    /** {@link #foreignType} of each of {@code ts}. */
    @SuppressWarnings("unchecked")
    private static List<Map<String, Object>> foreignTypes(List<Map<String, Object>> ts, String pkg) {
        List<Map<String, Object>> out = new ArrayList<>();
        for (Map<String, Object> t : ts) {
            out.add((Map<String, Object>) foreignType(t, pkg));
        }
        return out;
    }

    /**
     * A type of another package's module as this module names it: a struct, enum or interface of
     * that package, or a pointer to one, is the extern type of the package.
     */
    @SuppressWarnings("unchecked")
    private static Object foreignType(Object t, String pkg) {
        if (t instanceof Map) {
            Map<String, Object> m = (Map<String, Object>) t;
            if ("ptr".equals(m.get("k")) && m.get("elem") instanceof Map && "named".equals(((Map<String, Object>) m.get("elem")).get("k"))) {
                return foreignType(m.get("elem"), pkg);
            }
            if ("named".equals(m.get("k"))) {
                Map<String, Object> x = Json.obj();
                x.put("k", "extern");
                x.put("path", pkg);
                x.put("name", m.get("name"));
                return x;
            }
            Map<String, Object> out = Json.obj();
            for (Map.Entry<String, Object> en : m.entrySet()) {
                out.put(en.getKey(), foreignType(en.getValue(), pkg));
            }
            return out;
        }
        if (t instanceof List) {
            List<Object> out = new ArrayList<>();
            for (Object o : (List<Object>) t) {
                out.add(foreignType(o, pkg));
            }
            return out;
        }
        return t;
    }

    /** Take in the units of one package and classify its declarations. */
    private void prepare(List<CompilationUnitTree> units) throws Failure {
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
        pkgName = pkg;
        if (project) {
            nestedClasses();
        }
        classify();
    }

    /**
     * The classes nested in the top-level classes of a project read: each static class is read as a
     * struct of its own simple name, after the class it is nested in; an inner class, a nested
     * interface, enum or record, or a nested class whose simple name another type of the package has,
     * is refused at its line ({@link #nestedWhy}). A static nested class has no object of the
     * enclosing class, so nothing of it is lost; an inner class's objects hold one, which a cir/1
     * struct has no field for.
     */
    private void nestedClasses() {
        Set<String> names = new HashSet<>();
        for (TreePath p : decls) {
            names.add(classTree(p).getSimpleName().toString());
        }
        List<TreePath> all = new ArrayList<>();
        for (TreePath p : decls) {
            all.add(p);
            if (element(p).getKind() != ElementKind.CLASS) {
                continue;
            }
            for (Tree m : classTree(p).getMembers()) {
                if (!(m instanceof ClassTree)) {
                    continue;
                }
                TreePath np = new TreePath(p, m);
                TypeElement ne = element(np);
                String n = ne.getSimpleName().toString();
                String why = null;
                if (ne.getKind() != ElementKind.CLASS) {
                    why = "nested " + ne.getKind().toString().toLowerCase(Locale.ROOT).replace('_', ' ')
                        + ": only a static nested class is read";
                } else if (!ne.getModifiers().contains(Modifier.STATIC)) {
                    why = "inner class: each object holds the object of " + element(p).getSimpleName()
                        + " it was made in, which a cir/1 struct has no field for";
                } else if (names.contains(n)) {
                    why = "a nested class named as another type of the package: the types of a cir/1 module have one name each";
                }
                if (why != null) {
                    nestedWhy.put(m, why);
                    continue;
                }
                names.add(n);
                nested.add(m);
                all.add(np);
            }
        }
        decls.clear();
        decls.addAll(all);
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
        Map<TypeElement, String> unionWhy = new HashMap<>();
        Set<TypeElement> variants = new HashSet<>();
        if (!project) {
            for (TreePath p : decls) {
                TypeElement te = element(p);
                if (te.getKind() == ElementKind.INTERFACE && te.getModifiers().contains(Modifier.SEALED)) {
                    String why = unionShape(te, classTree(p));
                    unionWhy.put(te, why);
                    if (why == null) {
                        List<TypeElement> rs = new ArrayList<>();
                        for (TypeMirror s : te.getPermittedSubclasses()) {
                            rs.add((TypeElement) types.asElement(s));
                        }
                        unions.put(te, rs);
                        variants.addAll(rs);
                    }
                }
            }
        }
        // Shapes that do not depend on other declarations.
        for (TreePath p : decls) {
            TypeElement te = element(p);
            ClassTree ct = classTree(p);
            CompilationUnitTree cu = p.getCompilationUnit();
            boolean mapper = project && te.getKind() == ElementKind.CLASS && exceptionMapper(te);
            if (mapper && mapperShape(te) != null) {
                refuseDecl(cu, ct, ct.getModifiers(), te.getSimpleName().toString(), "a class implementing ExceptionMapper "
                    + mapperShape(te));
                refused.add(te);
                continue;
            }
            refuseAnnotations(cu, ct.getModifiers(), te.getKind() == ElementKind.CLASS, mapper ? Set.of(PROVIDER) : Set.of());
            String why = null;
            switch (te.getKind()) {
                case ANNOTATION_TYPE:
                    why = "annotation type: annotations with behaviour are outside cir/1";
                    break;
                case RECORD:
                    if (project) {
                        why = te.getKind() + " is outside cir/1";
                    } else if (!variants.contains(te)) {
                        why = "record other than a variant of a sealed interface: a cir/1 struct is mutable and compared by"
                            + " identity through its pointer";
                    }
                    break;
                case INTERFACE:
                    if (project && annotation(te, REST_CLIENT) != null) {
                        break;
                    }
                    if (unionWhy.containsKey(te)) {
                        why = unionWhy.get(te);
                        if (why == null && !READ_UNIONS) {
                            unions.remove(te);
                            why = "sealed interface of records: a cir/1 union, which the reader does not emit while the Go writer"
                                + " that checks a read refuses every union";
                        }
                        break;
                    }
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
                String format = msg == null && !project ? errorLeafFormat(p) : null;
                if (format != null) {
                    formattedLeaves.put(te, format);
                    continue;
                }
                if (msg == null) {
                    why = "exception class other than one constructor without parameters calling super(\"<message>\")"
                        + " or one calling super(String.format(\"<format>\", <its parameters in order>))";
                } else {
                    errorLeaves.put(te, msg);
                    continue;
                }
            } else if (!sup.equals("java.lang.Object") && !externalSuperclass(te)) {
                why = "class extends " + sup + ": inheritance beyond one checked-exception hierarchy is outside cir/1";
            } else {
                for (TypeMirror i : te.getInterfaces()) {
                    TypeElement ie = (TypeElement) types.asElement(i);
                    if (externalInterface(ie)) {
                        continue;
                    }
                    if (!interfaces.contains(ie)) {
                        why = "implements " + qname(i) + ": only an interface of the package with one implementation is read";
                        break;
                    }
                    if (implOf.containsKey(ie) && project) {
                        why = "a second implementation of interface " + ie.getSimpleName() + " (the first is "
                            + implOf.get(ie).getSimpleName() + "): interfaces with more than one implementation are outside cir/1";
                        implOf.remove(ie);
                        interfaces.remove(ie);
                        break;
                    }
                }
                if (why == null) {
                    for (TypeMirror i : te.getInterfaces()) {
                        TypeElement ie = (TypeElement) types.asElement(i);
                        if (externalInterface(ie)) {
                            continue;
                        }
                        if (implOf.containsKey(ie) || impls.containsKey(ie)) {
                            // A second implementation: the interface is a cir/1 interface of them all.
                            List<TypeElement> all = impls.computeIfAbsent(ie, k -> new ArrayList<>());
                            if (implOf.containsKey(ie)) {
                                all.add(implOf.remove(ie));
                            }
                            all.add(te);
                        } else {
                            implOf.put(ie, te);
                        }
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
                    for (List<TypeElement> all : impls.values()) {
                        all.remove(te);
                    }
                    refuseDecl(p.getCompilationUnit(), classTree(p), classTree(p).getModifiers(), te.getSimpleName().toString(), why);
                    changed = true;
                }
            }
            for (TreePath p : decls) {
                TypeElement te = element(p);
                if (!impls.containsKey(te)) {
                    continue;
                }
                String why = ifaceMethodsOf(te);
                if (why != null) {
                    impls.remove(te);
                    ifaceMethods.remove(te);
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
                if (extendsPanacheEntity(element(p))) {
                    unassigned.add(panacheId());
                }
            }
        }
        // A persistence annotation is a mark only on a struct: on a class read otherwise, or not at
        // all, it is refused like any other annotation.
        for (TreePath p : decls) {
            TypeElement te = element(p);
            if (project && te.getKind() == ElementKind.CLASS && !structs.contains(te) && !exceptionMapper(te)) {
                CompilationUnitTree cu = p.getCompilationUnit();
                for (AnnotationTree a : classTree(p).getModifiers().getAnnotations()) {
                    TypeMirror at = trees.getTypeMirror(new TreePath(new TreePath(cu), a.getAnnotationType()));
                    String qn = at == null ? "" : qname(at);
                    if (PERSISTENCE.contains(qn)) {
                        refuse(pos(cu, a), source(cu, a), "annotation " + qn + " on a class read as no struct: annotations"
                            + " with behaviour are outside cir/1, and a persistence mark is on a struct or its field");
                    }
                }
            }
        }
    }

    /** Whether a class of a project read is a JAX-RS {@code ExceptionMapper}. */
    private boolean exceptionMapper(TypeElement te) {
        TypeElement em = elements.getTypeElement(EXCEPTION_MAPPER);
        return em != null && types.isAssignable(types.erasure(te.asType()), types.erasure(em.asType()));
    }

    /**
     * Why an {@code ExceptionMapper} is no {@code exception-mapper} entry, or null when it is one:
     * registered by {@code @Provider}, declaring its own {@code toResponse}, whose parameter is a
     * jar's class, an extern type the module can name in {@code raises}.
     */
    private String mapperShape(TypeElement te) {
        if (annotation(te, PROVIDER) == null) {
            return "without @Provider: nothing in the source registers it, so the module cannot name the framework's call of"
                + " it as an entry";
        }
        ExecutableElement m = mapperMethod(te);
        if (m == null) {
            return "without its own toResponse(e): the entry names a function of the class";
        }
        TypeMirror t = m.getParameters().get(0).asType();
        if (t.getKind() != TypeKind.DECLARED || isProjectClass((TypeElement) types.asElement(t))) {
            return "of " + t + ", a class of the project: what a function raises, and so what the entry takes, is a jar's class";
        }
        return null;
    }

    /** The {@code toResponse(e)} an {@code ExceptionMapper} declares itself, or null. */
    private ExecutableElement mapperMethod(TypeElement te) {
        for (Element e : te.getEnclosedElements()) {
            if (e.getKind() == ElementKind.METHOD && e.getSimpleName().contentEquals("toResponse")
                && !e.getModifiers().contains(Modifier.STATIC) && ((ExecutableElement) e).getParameters().size() == 1) {
                return (ExecutableElement) e;
            }
        }
        return null;
    }

    /**
     * Whether an entry names the method: an http method of a {@code @Path} class, a {@code @Scheduled}
     * or {@code @Incoming} method, the {@code toResponse} of a registered {@code ExceptionMapper}.
     * {@link #framework} makes the entries.
     */
    private boolean entryMethod(ExecutableElement m) {
        if (!(m.getEnclosingElement() instanceof TypeElement) || m.getKind() != ElementKind.METHOD) {
            return false;
        }
        TypeElement te = (TypeElement) m.getEnclosingElement();
        if (annotation(m, SCHEDULED) != null || annotation(m, INCOMING) != null) {
            return true;
        }
        if (annotation(te, PATH) != null) {
            for (String verb : HTTP_METHODS) {
                if (annotation(m, "jakarta.ws.rs." + verb) != null) {
                    return true;
                }
            }
        }
        return te.getKind() == ElementKind.CLASS && exceptionMapper(te) && mapperShape(te) == null && m.equals(mapperMethod(te));
    }

    /**
     * Whether a method of the project may hold a {@code raise}: an entry names it and its body holds a
     * {@code throw} outside a lambda or a class body. A call of it is refused: what it raises leaves
     * every caller up to the framework, and only a function the framework calls lists it.
     */
    private boolean mayRaise(ExecutableElement m) {
        Boolean known = analysis.mayRaise.get(m);
        if (known != null) {
            return known;
        }
        boolean[] found = {false};
        TreePath mp = trees.getPath(m);
        if (mp != null && mp.getLeaf() instanceof MethodTree && ((MethodTree) mp.getLeaf()).getBody() != null && entryMethod(m)) {
            new TreeScanner<Void, Void>() {
                @Override
                public Void visitThrow(ThrowTree t, Void x) {
                    found[0] = true;
                    return null;
                }

                @Override
                public Void visitLambdaExpression(LambdaExpressionTree l, Void x) {
                    return null;
                }

                @Override
                public Void visitClass(ClassTree c, Void x) {
                    return null;
                }
            }.scan(((MethodTree) mp.getLeaf()).getBody(), null);
        }
        analysis.mayRaise.put(m, found[0]);
        return found[0];
    }

    /** Whether a method of the project's sources calls {@code m} or names it by a method reference. */
    private boolean calledInProject(ExecutableElement m) {
        if (analysis.called == null) {
            Set<Element> called = new HashSet<>();
            for (CompilationUnitTree cu : analysis.units) {
                new TreePathScanner<Void, Void>() {
                    @Override
                    public Void visitMethodInvocation(MethodInvocationTree t, Void x) {
                        called.add(trees.getElement(getCurrentPath()));
                        return super.visitMethodInvocation(t, x);
                    }

                    @Override
                    public Void visitMemberReference(MemberReferenceTree t, Void x) {
                        called.add(trees.getElement(getCurrentPath()));
                        return super.visitMemberReference(t, x);
                    }
                }.scan(cu, null);
            }
            analysis.called = called;
        }
        return analysis.called.contains(m);
    }

    /**
     * Whether a parameter of an http entry is a path parameter the framework alone passes: no
     * annotation, or {@code @PathParam} of its own name, a template variable {@code {name}} without a
     * regular expression in the entry's joined path, and no call of the method in the project's
     * sources. The framework hands it the non-empty segment it matched, converted, never null.
     */
    private boolean pathParameter(VariableElement v) {
        if (!project || !(v.getEnclosingElement() instanceof ExecutableElement)) {
            return false;
        }
        ExecutableElement m = (ExecutableElement) v.getEnclosingElement();
        TypeElement te = (TypeElement) m.getEnclosingElement();
        AnnotationMirror classPath = annotation(te, PATH);
        boolean http = false;
        for (String verb : HTTP_METHODS) {
            http |= annotation(m, "jakarta.ws.rs." + verb) != null;
        }
        if (classPath == null || !http) {
            return false;
        }
        String name = v.getSimpleName().toString();
        List<? extends AnnotationMirror> anns = v.getAnnotationMirrors();
        if (!anns.isEmpty() && !(anns.size() == 1 && name.equals(attribute(anns.get(0), "value"))
            && annotation(v, PATH_PARAM) != null)) {
            return false;
        }
        AnnotationMirror methodPath = annotation(m, PATH);
        String path = joinPaths(attribute(classPath, "value"), methodPath == null ? null : attribute(methodPath, "value"));
        return List.of(path.split("/")).contains("{" + name + "}") && !calledInProject(m);
    }

    /**
     * The field {@code id} of Panache's {@code PanacheEntity}, or null without it on the class path.
     * The jar declares it {@code public Long id} without an initializer, and no constructor of the
     * class assigns it: it is null until {@code persist()} gives the entity its key.
     */
    private VariableElement panacheId() {
        TypeElement pe = elements.getTypeElement(PANACHE_ENTITY);
        if (pe == null) {
            return null;
        }
        for (Element e : pe.getEnclosedElements()) {
            if (e.getKind() == ElementKind.FIELD && e.getSimpleName().contentEquals("id") && !e.getModifiers().contains(Modifier.STATIC)) {
                return (VariableElement) e;
            }
        }
        return null;
    }

    /**
     * Whether a class of a project read extends {@code PanacheEntity} directly: its struct holds the
     * inherited {@code id} before its own fields.
     */
    private boolean extendsPanacheEntity(TypeElement te) {
        return project && qname(te.getSuperclass()).equals(PANACHE_ENTITY) && panacheId() != null;
    }

    /**
     * Whether {@code te} is a Panache entity, a subclass of a Panache entity base.
     */
    private boolean panacheEntity(TypeElement te) {
        for (String b : PANACHE_ENTITY_BASES) {
            TypeElement be = elements.getTypeElement(b);
            if (be != null && types.isAssignable(types.erasure(te.asType()), types.erasure(be.asType()))) {
                return true;
            }
        }
        return false;
    }

    /**
     * Why a sealed interface is not a cir/1 union, or {@code null}: no members, no type parameters,
     * no super-interface, and every permitted class a record of the package implementing only it,
     * with no type parameters, no member of its own and components the reader reads.
     */
    private String unionShape(TypeElement te, ClassTree ct) {
        if (!ct.getTypeParameters().isEmpty()) {
            return "generic sealed interface: generics declared by the code are outside cir/1";
        }
        if (!ct.getImplementsClause().isEmpty()) {
            return "sealed interface extending another: inheritance is outside cir/1";
        }
        if (!ct.getMembers().isEmpty()) {
            return "sealed interface with members: a cir/1 union has variants only";
        }
        List<? extends TypeMirror> permitted = te.getPermittedSubclasses();
        if (permitted.isEmpty()) {
            return "sealed interface without permitted classes";
        }
        for (TypeMirror s : permitted) {
            TypeElement r = (TypeElement) types.asElement(s);
            String what = "permitted class " + r.getSimpleName();
            if (r.getKind() != ElementKind.RECORD || !inPackage(r)) {
                return what + " is not a record of the package: only records are cir/1 union variants";
            }
            if (!r.getTypeParameters().isEmpty()) {
                return what + " is generic: generics declared by the code are outside cir/1";
            }
            if (r.getInterfaces().size() != 1) {
                return what + " implements another interface: a cir/1 union variant is no other type";
            }
            TreePath rp = trees.getPath(r);
            // javac gives the implicit members (accessors, toString, equals, hashCode) no origin
            // of their own, so the body's source is what says the record declares nothing.
            if (!source(rp.getCompilationUnit(), rp.getLeaf()).replaceAll("\\s+", "").endsWith("{}")) {
                return what + " declares members: a cir/1 union variant has fields only";
            }
            for (javax.lang.model.element.RecordComponentElement c : r.getRecordComponents()) {
                try {
                    typeOf(c.asType());
                } catch (Unsupported u) {
                    return what + " component " + c.getSimpleName() + ": " + u.getMessage();
                }
            }
        }
        return null;
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

    /**
     * The methods of an interface with several implementations, as cir/1 interface methods sorted
     * by name, into {@link #ifaceMethods}; or why the interface is not read.
     */
    /**
     * The cir/1 name of an interface method: its Java name spelled exported, as a cir/1 interface
     * method's spelling says it is (Go's convention); every method of a Java interface is public,
     * and so is each implementation, which writers export under that same spelling.
     */
    static String ifaceMethodName(String javaName) {
        return javaName.isEmpty() ? javaName : Character.toUpperCase(javaName.charAt(0)) + javaName.substring(1);
    }

    private String ifaceMethodsOf(TypeElement ie) {
        TreeMap<String, Map<String, Object>> byName = new TreeMap<>();
        for (Element e : ie.getEnclosedElements()) {
            if (e.getKind() != ElementKind.METHOD) {
                continue;
            }
            ExecutableElement m = (ExecutableElement) e;
            String name = m.getSimpleName().toString();
            if (byName.containsKey(ifaceMethodName(name))) {
                return "a second method named " + ifaceMethodName(name) + ": overloading is outside cir/1";
            }
            if (!m.getThrownTypes().isEmpty() || m.isVarArgs() || qname(m.getReturnType()).equals("java.util.Optional")) {
                return "interface method " + name + " that throws, takes variable arity or returns an Optional";
            }
            List<Object> params = new ArrayList<>();
            List<Object> results = new ArrayList<>();
            try {
                for (VariableElement v : m.getParameters()) {
                    params.add(typeOf(v.asType()));
                }
                if (m.getReturnType().getKind() != TypeKind.VOID) {
                    results.add(typeOf(m.getReturnType()));
                }
            } catch (Unsupported u) {
                return "interface method " + name + ": " + u.getMessage();
            }
            Map<String, Object> x = Json.obj();
            x.put("name", ifaceMethodName(name));
            x.put("params", params);
            x.put("results", results);
            byName.put(ifaceMethodName(name), x);
        }
        ifaceMethods.put(ie, new ArrayList<>(byName.values()));
        return null;
    }

    /** The interface with several implementations that the named type {@code t} is, or null. */
    private TypeElement iface(Map<String, Object> t) {
        if (!"named".equals(t.get("k"))) {
            return null;
        }
        for (TypeElement ie : impls.keySet()) {
            if (ie.getSimpleName().contentEquals((String) t.get("name"))) {
                return ie;
            }
        }
        return null;
    }

    /**
     * {@code e}, an object of a class implementing {@code ie}, used where a value of {@code ie} is
     * expected: every method of the interface must be a method the module reads of that class,
     * with the interface's signature, or the class does not implement the cir/1 interface.
     */
    private Map<String, Object> toIface(Map<String, Object> e, TypeElement ie, Map<String, Object> want) {
        Map<String, Object> t = type(e);
        if (!"ptr".equals(t.get("k")) || !"named".equals(sub(t, "elem").get("k"))) {
            throw new Unsupported("a " + describe(t) + " where interface " + ie.getSimpleName() + " is expected");
        }
        String cls = (String) sub(t, "elem").get("name");
        TypeElement impl = null;
        for (TypeElement s : impls.get(ie)) {
            if (s.getSimpleName().contentEquals(cls)) {
                impl = s;
            }
        }
        if (impl == null) {
            throw new Unsupported("class " + cls + " does not implement interface " + ie.getSimpleName());
        }
        for (Map<String, Object> m : ifaceMethods.get(ie)) {
            Fn found = null;
            for (Fn fn : fns.values()) {
                if (fn.cls.equals(impl) && !fn.isStatic && !fn.isCtor && fn.exported && ifaceMethodName(fn.name).equals(m.get("name"))) {
                    found = fn;
                }
            }
            if (found == null || !new ArrayList<Object>(found.params).equals(m.get("params"))
                || !new ArrayList<Object>(found.results()).equals(m.get("results"))) {
                throw new Unsupported("class " + cls + " does not have the method " + m.get("name") + " of interface "
                    + ie.getSimpleName() + " as the module reads it");
            }
        }
        Map<String, Object> x = kind("to_iface");
        x.put("x", e);
        x.put("t", want);
        return x;
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
     * The format of an exception leaf whose one constructor takes parameters and only calls {@code
     * super(String.format("<format>", p1, …, pn))} with its parameters, each once, in order; else
     * null. A throw of it is that format applied to the throw's arguments, which Java evaluates in
     * order before it formats them, as {@code errorf}'s arguments are.
     */
    private String errorLeafFormat(TreePath p) {
        ClassTree ct = classTree(p);
        if (!ct.getImplementsClause().isEmpty() || !ct.getTypeParameters().isEmpty()) {
            return null;
        }
        String format = null;
        for (Tree m : ct.getMembers()) {
            if (isSerialVersionUid(p, m)) {
                continue;
            }
            if (!(m instanceof MethodTree)) {
                return null;
            }
            MethodTree mt = (MethodTree) m;
            Element e = trees.getElement(new TreePath(p, m));
            if (e.getKind() != ElementKind.CONSTRUCTOR || generated(e) || format != null || mt.getParameters().isEmpty()) {
                return null;
            }
            List<? extends StatementTree> body = ctorStatements(mt);
            ExpressionTree arg = body.size() == 1 ? superArgument(body.get(0)) : null;
            if (!(arg instanceof MethodInvocationTree)) {
                return null;
            }
            MethodInvocationTree mi = (MethodInvocationTree) arg;
            Element called = trees.getElement(new TreePath(new TreePath(new TreePath(new TreePath(p, m), mt.getBody()), body.get(0)), arg));
            if (!(called instanceof ExecutableElement) || !called.getSimpleName().contentEquals("format")
                || !((TypeElement) called.getEnclosingElement()).getQualifiedName().contentEquals("java.lang.String")
                || mi.getArguments().size() != mt.getParameters().size() + 1
                || mi.getArguments().get(0).getKind() != Tree.Kind.STRING_LITERAL) {
                return null;
            }
            for (int i = 0; i < mt.getParameters().size(); i++) {
                ExpressionTree a = mi.getArguments().get(i + 1);
                if (!(a instanceof IdentifierTree) || !((IdentifierTree) a).getName().equals(mt.getParameters().get(i).getName())) {
                    return null;
                }
            }
            format = (String) ((LiteralTree) mi.getArguments().get(0)).getValue();
        }
        return format;
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
            if (project && annotation(e, INJECT) != null) {
                continue;
            }
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
        boolean inheritsId = extendsPanacheEntity(element(p));
        for (Tree m : classTree(p).getMembers()) {
            if (m instanceof VariableTree) {
                Element e = trees.getElement(new TreePath(p, m));
                if (e.getModifiers().contains(Modifier.STATIC)) {
                    continue;
                }
                if (inheritsId && e.getSimpleName().contentEquals("id")) {
                    return "field id hides PanacheEntity.id: a cir/1 struct has one field of a name";
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
            boolean declaresCtor = false;
            for (Tree m : classTree(p).getMembers()) {
                TreePath mp = new TreePath(p, m);
                if (m instanceof ClassTree) {
                    if (nested.contains(m)) {
                        continue;
                    }
                    String why = !project ? "nested class: only top-level classes are read"
                        : nestedWhy.containsKey(m) ? nestedWhy.get(m)
                        : "a class nested in a nested class: only a class nested in a top-level class is read";
                    refuseDecl(cu, m, ((ClassTree) m).getModifiers(), ((ClassTree) m).getSimpleName().toString(), why);
                    continue;
                }
                if (m instanceof VariableTree) {
                    VariableTree vt = (VariableTree) m;
                    VariableElement ve = (VariableElement) trees.getElement(mp);
                    refuseAnnotations(cu, vt.getModifiers(), !ve.getModifiers().contains(Modifier.STATIC));
                    if (!ve.getModifiers().contains(Modifier.STATIC)) {
                        continue;
                    }
                    String why = null;
                    String vname = ve.getSimpleName().toString();
                    boolean mutable = !ve.getModifiers().contains(Modifier.FINAL);
                    if (mutable && !project) {
                        why = varShape(vt, ve);
                        if (why == null && (funcNames.contains(vname) || consts.containsValue(vname) || vars.containsValue(vname))) {
                            why = "a second module-level name " + vname;
                        }
                        if (why == null) {
                            vars.put(ve, vname);
                        } else {
                            refuseDecl(cu, m, vt.getModifiers(), te.getSimpleName() + "." + vt.getName(), why);
                        }
                        continue;
                    }
                    if (project && !mutable && ve.getConstantValue() == null) {
                        why = staticFinalShape(mp, vt, ve);
                        if (why == null && (funcNames.contains(vname) || consts.containsValue(vname) || vars.containsValue(vname))) {
                            why = "a second module-level name " + vname;
                        }
                        if (why == null) {
                            vars.put(ve, vname);
                        } else {
                            refuseDecl(cu, m, vt.getModifiers(), te.getSimpleName() + "." + vt.getName(), why);
                        }
                        continue;
                    }
                    if (!ve.getModifiers().contains(Modifier.FINAL) || ve.getConstantValue() == null) {
                        why = "static field other than a compile-time constant: module-level state is outside cir/1";
                    } else if (funcNames.contains(ve.getSimpleName().toString()) || consts.containsValue(ve.getSimpleName().toString())
                        || vars.containsValue(ve.getSimpleName().toString())) {
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
                declaresCtor |= ctor;
                String name = ctor ? (project ? ctorName(ee) : "New" + te.getSimpleName()) : ee.getSimpleName().toString();
                String what = ctor ? te.getSimpleName() + "(…)" : te.getSimpleName() + "." + name + "(…)";
                String why = null;
                if (ctor && project && callsThis(mt)) {
                    why = "a constructor calling this(…): Java runs the field initializers once, in the constructor it calls,"
                        + " and cir/1 has no call of a constructor on an object already made";
                } else if (ctor && explicitCtor && !project) {
                    why = "a second constructor: overloading is outside cir/1";
                } else if (ctor && project && !name.equals("New" + te.getSimpleName()) && funcNames.contains(name)) {
                    why = "a second function named " + name + " in the package";
                } else if (!ctor && methodNames.contains(name)) {
                    why = "a second method named " + name + ": overloading is outside cir/1";
                } else if (!ctor && ee.getModifiers().contains(Modifier.STATIC) && funcNames.contains(name)) {
                    why = "a second function named " + name + " in the package";
                } else if (!ctor && ee.getModifiers().contains(Modifier.STATIC) && vars.containsValue(name)) {
                    why = "a function named as the module-level variable " + name;
                } else if (!ee.getTypeParameters().isEmpty() && (project || !ee.getModifiers().contains(Modifier.STATIC))) {
                    why = "generic method other than a static one of a package read: generics declared by the code are outside cir/1";
                } else if (!ee.getTypeParameters().isEmpty() && !unbounded(ee)) {
                    why = "generic method with a bounded type parameter: only type parameters over any are read";
                } else if (mt.getBody() == null) {
                    why = "method without a body";
                } else if (ee.isVarArgs() && project) {
                    why = "variable arity method";
                }
                if (why == null && !ee.getTypeParameters().isEmpty()) {
                    genericMethods.add(ee);
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
                    if (!explicitCtor) {
                        ctors.put(te, fn);
                    }
                    explicitCtor = true;
                    funcNames.add(name);
                } else {
                    methodNames.add(name);
                    if (fn.isStatic) {
                        funcNames.add(name);
                    }
                }
            }
            if (!explicitCtor && !(project && declaresCtor)) {
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

    /**
     * The function a project read makes of a constructor: {@code New<Class>} for the first the class
     * declares, {@code New<Class><n>} for the n-th after it, in source order, read or refused; a
     * call names the one javac chose. The default constructor javac adds is {@code New<Class>}.
     */
    private String ctorName(ExecutableElement ce) {
        TypeElement te = (TypeElement) ce.getEnclosingElement();
        int i = 0;
        for (Element e : te.getEnclosedElements()) {
            if (e.getKind() != ElementKind.CONSTRUCTOR || generated(e)) {
                continue;
            }
            if (e.equals(ce)) {
                break;
            }
            i++;
        }
        return "New" + te.getSimpleName() + (i == 0 ? "" : String.valueOf(i + 1));
    }

    /** Whether a constructor starts by calling another of its class, {@code this(…)}. */
    private static boolean callsThis(MethodTree mt) {
        List<? extends StatementTree> body = mt.getBody() == null ? List.of() : mt.getBody().getStatements();
        if (body.isEmpty() || !(body.get(0) instanceof ExpressionStatementTree)) {
            return false;
        }
        ExpressionTree e = ((ExpressionStatementTree) body.get(0)).getExpression();
        return e instanceof MethodInvocationTree && ((MethodInvocationTree) e).getMethodSelect() instanceof IdentifierTree
            && ((IdentifierTree) ((MethodInvocationTree) e).getMethodSelect()).getName().contentEquals("this");
    }

    /**
     * Why a {@code static final} field of a project read that is not a compile-time constant is not
     * read as a module-level variable, or null. Java runs its initializer when it first uses the
     * class, cir/1 when it loads the module; the value is the same when the initializer is a literal,
     * or a call of a static method of a jar whose arguments are literals, constants or {@code
     * C.class.getName()}, and the method gives the same value whenever it is called (README.md,
     * Assumptions). A call of the module's own code or of a value computed at run time could see
     * another state at load than at first use.
     */
    private String staticFinalShape(TreePath fp, VariableTree vt, VariableElement ve) {
        try {
            typeOf(ve.asType());
        } catch (Unsupported u) {
            return u.getMessage();
        }
        ExpressionTree init = vt.getInitializer();
        String why = "a static field initialized other than by a literal or by a call of a jar's static method with constant"
            + " arguments: Java runs the initializer when it first uses the class, a module-level variable is set when the"
            + " module is loaded";
        if (init == null || init.getKind() == Tree.Kind.NULL_LITERAL || init.getKind() == Tree.Kind.CHAR_LITERAL) {
            return why;
        }
        TreePath ip = new TreePath(fp, init);
        if (!(init instanceof LiteralTree)) {
            if (!(init instanceof MethodInvocationTree) || !(trees.getElement(ip) instanceof ExecutableElement)) {
                return why;
            }
            ExecutableElement m = (ExecutableElement) trees.getElement(ip);
            TypeElement owner = (TypeElement) m.getEnclosingElement();
            if (!m.getModifiers().contains(Modifier.STATIC) || isProjectClass(owner) || owner.getQualifiedName().toString().startsWith("java.")) {
                return why;
            }
            for (ExpressionTree a : ((MethodInvocationTree) init).getArguments()) {
                TreePath ap = new TreePath(ip, a);
                Element ae = trees.getElement(ap);
                boolean constant = a instanceof LiteralTree && a.getKind() != Tree.Kind.NULL_LITERAL
                    || ae instanceof VariableElement && ((VariableElement) ae).getConstantValue() != null
                    || classNameOf(ap) != null;
                if (!constant) {
                    return why;
                }
            }
        }
        try {
            Map<String, Object> t = typeOf(ve.asType());
            checkTyped(coerce(expr(ip, new Ctx(null, fp.getCompilationUnit(), false)), t));
        } catch (Unsupported u) {
            return u.getMessage();
        }
        return null;
    }

    /**
     * The binary name of the class {@code C} when {@code p} is {@code C.class.getName()} of a class
     * or interface, which is the name {@code getName} returns; else null.
     */
    private String classNameOf(TreePath p) {
        if (!(p.getLeaf() instanceof MethodInvocationTree)) {
            return null;
        }
        MethodInvocationTree mi = (MethodInvocationTree) p.getLeaf();
        if (!mi.getArguments().isEmpty() || !(mi.getMethodSelect() instanceof MemberSelectTree)) {
            return null;
        }
        MemberSelectTree sel = (MemberSelectTree) mi.getMethodSelect();
        if (!sel.getIdentifier().contentEquals("getName") || !(sel.getExpression() instanceof MemberSelectTree)
            || !((MemberSelectTree) sel.getExpression()).getIdentifier().contentEquals("class")) {
            return null;
        }
        ExpressionTree of = ((MemberSelectTree) sel.getExpression()).getExpression();
        TypeMirror t = trees.getTypeMirror(new TreePath(new TreePath(new TreePath(p, sel), sel.getExpression()), of));
        if (t == null || t.getKind() != TypeKind.DECLARED) {
            return null;
        }
        return elements.getBinaryName((TypeElement) ((DeclaredType) t).asElement()).toString();
    }

    /**
     * Why a static field that is not final is not read as a module-level variable, or null: its
     * initializer is absent, for a type whose zero value is Java's default (an integer, a boolean,
     * a double or a float), or a literal, so when Java initializes the class does not matter.
     */
    private String varShape(VariableTree vt, VariableElement ve) {
        Map<String, Object> t;
        try {
            t = typeOf(ve.asType());
        } catch (Unsupported u) {
            return u.getMessage();
        }
        ExpressionTree init = vt.getInitializer();
        if (init == null) {
            return isInt(t) || isFloat(t) || t.equals(tBool()) ? null
                : "a static field of a reference type without an initializer holds null, which no cir/1 zero value of "
                    + describe(t) + " is";
        }
        if (!(init instanceof LiteralTree) || init.getKind() == Tree.Kind.NULL_LITERAL || init.getKind() == Tree.Kind.CHAR_LITERAL) {
            return "a static field initialized other than by a literal: Java runs the initializer when it first uses the class";
        }
        return null;
    }

    /** A module-level variable: its type, and its literal initializer converted to that type. */
    private void emitVar(CompilationUnitTree cu, VariableTree vt, VariableElement ve) {
        Map<String, Object> t = typeOf(ve.asType());
        Map<String, Object> d = Json.obj();
        d.put("name", vars.get(ve));
        d.put("t", t);
        if (vt.getInitializer() != null) {
            Ctx none = new Ctx(null, cu, false);
            d.put("init", coerce(expr(new TreePath(trees.getPath(ve), vt.getInitializer()), none), t));
        }
        d.put("pos", declPos(cu, vt, vt.getModifiers()));
        outVars.add(d);
        layout.add("var:" + vars.get(ve));
    }

    private Fn signature(TypeElement te, ExecutableElement ee, TreePath mp, String name) {
        Fn fn = new Fn();
        fn.name = name;
        fn.exported = ee.getModifiers().contains(Modifier.PUBLIC);
        fn.isStatic = ee.getModifiers().contains(Modifier.STATIC);
        fn.isCtor = ee.getKind() == ElementKind.CONSTRUCTOR;
        fn.cls = te;
        fn.path = mp;
        fn.variadic = ee.isVarArgs();
        for (javax.lang.model.element.TypeParameterElement tp : ee.getTypeParameters()) {
            fn.tparams.add(tp.getSimpleName().toString());
        }
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

    /** Whether every type parameter of a method is bounded by {@code Object} alone: Go's {@code any}. */
    private static boolean unbounded(ExecutableElement ee) {
        for (javax.lang.model.element.TypeParameterElement tp : ee.getTypeParameters()) {
            List<? extends TypeMirror> bounds = tp.getBounds();
            if (bounds.size() != 1 || !(bounds.get(0) instanceof DeclaredType)
                || !((TypeElement) ((DeclaredType) bounds.get(0)).asElement()).getQualifiedName().contentEquals("java.lang.Object")) {
                return false;
            }
        }
        return true;
    }

    private boolean isErrorType(TypeMirror t) {
        if (t.getKind() != TypeKind.DECLARED) {
            return false;
        }
        TypeElement te = (TypeElement) types.asElement(t);
        return te.equals(errorRoot) || errorLeaves.containsKey(te) || formattedLeaves.containsKey(te);
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
        if (unions.containsKey(te)) {
            Map<String, Object> d = Json.obj();
            d.put("k", "union");
            d.put("name", name);
            d.put("exported", te.getModifiers().contains(Modifier.PUBLIC));
            List<Object> vs = new ArrayList<>();
            for (TypeElement r : unions.get(te)) {
                Map<String, Object> v = Json.obj();
                v.put("name", r.getSimpleName().toString());
                List<Object> fields = new ArrayList<>();
                for (javax.lang.model.element.RecordComponentElement c : r.getRecordComponents()) {
                    Map<String, Object> f = Json.obj();
                    f.put("name", c.getSimpleName().toString());
                    f.put("t", typeOf(c.asType()));
                    fields.add(f);
                }
                v.put("fields", fields);
                vs.add(v);
            }
            d.put("variants", vs);
            putDoc(d, te);
            d.put("pos", declPos(cu, ct, ct.getModifiers()));
            outTypes.add(d);
            layout.add("type:" + name);
            return;
        }
        if (impls.containsKey(te)) {
            Map<String, Object> d = Json.obj();
            d.put("name", name);
            d.put("exported", te.getModifiers().contains(Modifier.PUBLIC));
            d.put("methods", new ArrayList<Object>(ifaceMethods.get(te)));
            putDoc(d, te);
            d.put("pos", declPos(cu, ct, ct.getModifiers()));
            outInterfaces.add(d);
            layout.add("type:" + name);
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
        if (extendsPanacheEntity(te)) {
            Map<String, Object> f = Json.obj();
            f.put("name", "id");
            f.put("t", fieldType(panacheId()));
            fields.add(f);
        }
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
        if (ctor != null && ctor.path == p) {
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
            } else if (m instanceof VariableTree && vars.containsKey(e)) {
                emitVar(cu, (VariableTree) m, (VariableElement) e);
            } else if (m instanceof MethodTree && fns.containsKey(e)) {
                emitFunc(fns.get(e));
            }
        }
        if (project) {
            persistence(cu, name, ct.getModifiers());
            for (Tree m : ct.getMembers()) {
                if (m instanceof VariableTree && !trees.getElement(new TreePath(p, m)).getModifiers().contains(Modifier.STATIC)) {
                    persistence(cu, name + "." + ((VariableTree) m).getName(), ((VariableTree) m).getModifiers());
                }
            }
            framework(p, te, ct);
        }
    }

    /**
     * Java source text on one line: comments dropped, and every run of white space and comments
     * outside a literal one space. A string, character or text block literal is copied byte for
     * byte, so a tab or a text block's line break stays in it.
     */
    private static String oneLine(String src) {
        StringBuilder b = new StringBuilder();
        boolean space = false;
        int i = 0;
        int n = src.length();
        while (i < n) {
            char c = src.charAt(i);
            if (Character.isWhitespace(c)) {
                space = true;
                i++;
                continue;
            }
            if (c == '/' && i + 1 < n && src.charAt(i + 1) == '/') {
                while (i < n && src.charAt(i) != '\n' && src.charAt(i) != '\r') {
                    i++;
                }
                space = true;
                continue;
            }
            if (c == '/' && i + 1 < n && src.charAt(i + 1) == '*') {
                int end = src.indexOf("*/", i + 2);
                i = end < 0 ? n : end + 2;
                space = true;
                continue;
            }
            if (space && b.length() > 0) {
                b.append(' ');
            }
            space = false;
            int end = i + 1;
            if (src.startsWith("\"\"\"", i)) {
                end = literalEnd(src, i + 3, "\"\"\"");
            } else if (c == '"' || c == '\'') {
                end = literalEnd(src, i + 1, String.valueOf(c));
            }
            b.append(src, i, end);
            i = end;
        }
        return b.toString();
    }

    /** The index after the {@code close} that ends a literal whose content starts at {@code from}, skipping escapes. */
    private static int literalEnd(String src, int from, String close) {
        int i = from;
        while (i < src.length()) {
            if (src.charAt(i) == '\\') {
                i += 2;
            } else if (src.startsWith(close, i)) {
                return i + close.length();
            } else {
                i++;
            }
        }
        return src.length();
    }

    /** The JPA annotations of a struct or of one of its fields, {@code at}, as {@code persistence} marks. */
    private void persistence(CompilationUnitTree cu, String at, ModifiersTree mods) {
        for (AnnotationTree a : mods.getAnnotations()) {
            TypeMirror t = trees.getTypeMirror(new TreePath(new TreePath(cu), a.getAnnotationType()));
            String qn = t == null ? "" : qname(t);
            if (!PERSISTENCE.contains(qn)) {
                continue;
            }
            Map<String, Object> x = Json.obj();
            x.put("at", at);
            x.put("mark", qn.substring(qn.lastIndexOf('.') + 1));
            List<Object> args = new ArrayList<>();
            for (ExpressionTree e : a.getArguments()) {
                String arg = oneLine(source(cu, e));
                if (arg.chars().anyMatch(Character::isISOControl)) {
                    refuse(pos(cu, a), source(cu, a), "annotation " + qn + " with an argument holding a control character in"
                        + " a literal: a persistence mark's arguments are one line each");
                    args = null;
                    break;
                }
                args.add(arg);
            }
            if (args == null) {
                continue;
            }
            if (!args.isEmpty()) {
                x.put("args", args);
            }
            outPersistence.add(x);
        }
    }

    /**
     * What Quarkus makes of the class: a bean, its injection points, and its resource, scheduled and
     * consumer methods as entry points. Only the methods the module reads are entry points.
     */
    private void framework(TreePath p, TypeElement te, ClassTree ct) {
        CompilationUnitTree cu = p.getCompilationUnit();
        String cls = te.getSimpleName().toString();
        String sc = scope(te);
        if (sc != null) {
            Map<String, Object> b = Json.obj();
            b.put("name", cls);
            b.put("scope", sc);
            b.put("pos", declPos(cu, ct, ct.getModifiers()));
            outBeans.add(b);
        }
        AnnotationMirror classPath = annotation(te, PATH);
        for (Tree m : ct.getMembers()) {
            Element e = trees.getElement(new TreePath(p, m));
            if (m instanceof VariableTree && !e.getModifiers().contains(Modifier.STATIC) && annotation(e, INJECT) != null) {
                inject(cls + "." + e.getSimpleName(), fieldType((VariableElement) e), e.asType());
            } else if (m instanceof MethodTree && fns.containsKey(e)) {
                Fn fn = fns.get(e);
                ExecutableElement ee = (ExecutableElement) e;
                if (fn.isCtor) {
                    if (annotation(e, INJECT) != null) {
                        for (int i = 0; i < ee.getParameters().size(); i++) {
                            inject(cls + ".<new>." + fn.paramNames.get(i), fn.params.get(i), ee.getParameters().get(i).asType());
                        }
                    }
                    continue;
                }
                AnnotationMirror methodPath = annotation(e, PATH);
                AnnotationMirror scheduled = annotation(e, SCHEDULED);
                AnnotationMirror incoming = annotation(e, INCOMING);
                if (annotation(e, TRANSACTIONAL) != null || annotation(te, TRANSACTIONAL) != null) {
                    outTransactional.add(fn.key());
                }
                for (String verb : HTTP_METHODS) {
                    if (classPath != null && annotation(e, "jakarta.ws.rs." + verb) != null) {
                        entry("http", fn.key(), verb, joinPaths(attribute(classPath, "value"), methodPath == null ? null : attribute(methodPath, "value")),
                            media(e, te, CONSUMES), media(e, te, PRODUCES));
                    }
                }
                if (scheduled != null) {
                    String cron = attribute(scheduled, "cron");
                    String every = attribute(scheduled, "every");
                    entry("scheduled", fn.key(), "", cron != null && !cron.isEmpty() ? "cron=" + cron
                        : every != null ? "every=" + every : "");
                }
                if (incoming != null) {
                    String channel = attribute(incoming, "value");
                    entry("incoming", fn.key(), "", channel == null ? "" : channel);
                }
                Map<String, Object> taken = fn.params.size() == 1 ? fn.params.get(0) : null;
                if (te.getKind() == ElementKind.CLASS && exceptionMapper(te) && mapperShape(te) == null && ee.equals(mapperMethod(te))
                    && "extern".equals(taken.get("k")) && !taken.containsKey("targs")) {
                    entry("exception-mapper", fn.key(), "", taken.get("path") + "." + taken.get("name"));
                }
            }
        }
    }

    private void entry(String kind, String func, String method, String on) {
        entry(kind, func, method, on, List.of(), List.of());
    }

    private void entry(String kind, String func, String method, String on, List<String> consumes, List<String> produces) {
        Map<String, Object> x = Json.obj();
        x.put("kind", kind);
        x.put("func", func);
        if (!method.isEmpty()) {
            x.put("method", method);
        }
        x.put("on", on);
        if (!consumes.isEmpty()) {
            x.put("consumes", new ArrayList<Object>(consumes));
        }
        if (!produces.isEmpty()) {
            x.put("produces", new ArrayList<Object>(produces));
        }
        outEntries.add(x);
    }

    /** The media types of {@code ann} on the method, else on its class; empty when neither declares it. */
    private static List<String> media(Element method, TypeElement cls, String ann) {
        AnnotationMirror am = annotation(method, ann);
        if (am == null) {
            am = annotation(cls, ann);
        }
        List<String> out = new ArrayList<>();
        if (am == null) {
            return out;
        }
        for (Map.Entry<? extends ExecutableElement, ? extends javax.lang.model.element.AnnotationValue> en
            : am.getElementValues().entrySet()) {
            if (en.getKey().getSimpleName().contentEquals("value")) {
                Object v = en.getValue().getValue();
                if (v instanceof String) {
                    out.add((String) v);
                } else if (v instanceof List<?>) {
                    for (Object o : (List<?>) v) {
                        Object s = o instanceof javax.lang.model.element.AnnotationValue
                            ? ((javax.lang.model.element.AnnotationValue) o).getValue() : o;
                        if (s instanceof String) {
                            out.add((String) s);
                        }
                    }
                }
            }
        }
        return out;
    }

    private void inject(String at, Map<String, Object> t, TypeMirror declared) {
        Map<String, Object> x = Json.obj();
        x.put("at", at);
        x.put("t", t);
        String bean = resolveBean(declared);
        if (!bean.isEmpty()) {
            x.put("bean", bean);
        }
        outInjects.add(x);
    }

    /**
     * The one bean of the project that is a {@code t}, as its package and the name its module lists
     * it by; empty when none or several are, or when its package reads it as no struct and so lists
     * no bean of it.
     */
    private String resolveBean(TypeMirror t) {
        TypeMirror want = types.erasure(t);
        TypeElement found = null;
        int n = 0;
        for (TypeElement b : analysis.beans) {
            if (types.isAssignable(types.erasure(b.asType()), want)) {
                found = b;
                n++;
            }
        }
        if (n != 1 || !structs.contains(found) && !otherStructs.contains(found)) {
            return "";
        }
        return packageOf(found) + "." + found.getSimpleName();
    }

    /** The resource's path and the method's joined: every segment, one {@code /} between, a leading {@code /}. */
    private static String joinPaths(String resource, String method) {
        StringBuilder b = new StringBuilder();
        for (String part : new String[] {resource == null ? "" : resource, method == null ? "" : method}) {
            for (String seg : part.split("/")) {
                if (!seg.isEmpty()) {
                    b.append('/').append(seg);
                }
            }
        }
        return b.length() == 0 ? "/" : b.toString();
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
        if (!fn.tparams.isEmpty()) {
            List<Object> tps = new ArrayList<>();
            for (String tp : fn.tparams) {
                Map<String, Object> x = Json.obj();
                x.put("name", tp);
                x.put("constraint", kind("any"));
                tps.add(x);
            }
            f.put("tparams", tps);
        }
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
            if (fn.variadic && i == fn.params.size() - 1) {
                pm.put("variadic", Boolean.TRUE);
            }
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
        List<Object> raises = raisesOf(body);
        if (!raises.isEmpty()) {
            f.put("raises", raises);
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

    /** The body of a loop: {@code break} and {@code continue} inside it, by its label or none, leave it. */
    private List<Object> inside(String target, TreePath parent, StatementTree s, Ctx ctx) {
        ctx.breaks.push(target);
        ctx.loopLabels.push(ctx.nextLabel);
        ctx.nextLabel = "";
        try {
            return body(parent, s, ctx);
        } finally {
            ctx.breaks.pop();
            ctx.loopLabels.pop();
        }
    }

    /**
     * Whether a {@code break} or {@code continue} naming {@code label} names the innermost loop: a
     * cir/1 {@code break} or {@code continue} leaves or ends the innermost loop, from inside a
     * switch too, so a jump to that loop's label is one and a jump to an outer loop is not.
     */
    private static void innermostLoop(String label, String what, Ctx ctx) {
        if (!label.equals(ctx.loopLabels.peek())) {
            throw new Unsupported("labeled " + what + " of a loop other than the innermost: cir/1 " + what
                + " leaves the innermost loop");
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
                    if (project) {
                        throw new Unsupported("labeled break is outside cir/1");
                    }
                    innermostLoop(((BreakTree) s).getLabel().toString(), "break", ctx);
                    return List.of(simple("break", pos));
                }
                if (!"loop".equals(ctx.breaks.peek())) {
                    throw new Unsupported("break out of a switch other than at the end of a case: cir/1 break leaves a loop");
                }
                return List.of(simple("break", pos));
            }
            case CONTINUE:
                if (((ContinueTree) s).getLabel() != null) {
                    if (project) {
                        throw new Unsupported("labeled continue is outside cir/1");
                    }
                    innermostLoop(((ContinueTree) s).getLabel().toString(), "continue", ctx);
                }
                return List.of(simple("continue", pos));
            case LABELED_STATEMENT: {
                if (project) {
                    throw new Unsupported("labeled statement is outside cir/1");
                }
                LabeledStatementTree ls = (LabeledStatementTree) s;
                Tree.Kind k = ls.getStatement().getKind();
                if (k != Tree.Kind.WHILE_LOOP && k != Tree.Kind.FOR_LOOP && k != Tree.Kind.ENHANCED_FOR_LOOP) {
                    throw new Unsupported("label on a statement other than a while, for or for-each loop");
                }
                ctx.nextLabel = ls.getLabel().toString();
                try {
                    return strict(new TreePath(p, ls.getStatement()), ctx);
                } finally {
                    ctx.nextLabel = "";
                }
            }
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
            refuseUnassigned(ip, "the initializer of local " + name, ESCAPES);
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
                refuseUnassigned(new TreePath(ep, at.getExpression()), "assigned", ESCAPES);
                Element av = trees.getElement(new TreePath(ep, at.getVariable()));
                if (av != null && av.getKind() == ElementKind.LOCAL_VARIABLE && av.asType().getKind() == TypeKind.DECLARED && impls.containsKey((TypeElement) types.asElement(av.asType()))) {
                    throw new Unsupported("an assignment to " + av.getSimpleName() + ", a local of interface "
                        + types.asElement(av.asType()).getSimpleName() + ": the Go writer declares a local with the type of the"
                        + " value it starts with, so another implementation is not assignable to it");
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
            case XOR_ASSIGNMENT:
            case LEFT_SHIFT_ASSIGNMENT:
            case RIGHT_SHIFT_ASSIGNMENT: {
                if (project && (e.getKind() == Tree.Kind.LEFT_SHIFT_ASSIGNMENT || e.getKind() == Tree.Kind.RIGHT_SHIFT_ASSIGNMENT)) {
                    throw new Unsupported("expression statement " + e.getKind().toString().toLowerCase(Locale.ROOT).replace('_', ' ')
                        + " is outside cir/1");
                }
                CompoundAssignmentTree ct = (CompoundAssignmentTree) e;
                refuseUnassigned(new TreePath(ep, ct.getVariable()), "the target of " + compoundOp(e.getKind()),
                    "Java unboxes or prints the null" + STOPS);
                refuseUnassigned(new TreePath(ep, ct.getExpression()), "an operand of " + compoundOp(e.getKind()),
                    "Java unboxes or prints the null" + STOPS);
                Map<String, Object> lhs = place(new TreePath(ep, ct.getVariable()), ctx);
                String k = (String) lhs.get("k");
                if (("index".equals(k) || "field".equals(k) || "global".equals(k)) && callsCode(new TreePath(ep, ct.getExpression()))) {
                    throw new Unsupported(compoundOp(e.getKind()) + " on an array element, a field or a module-level variable with a"
                        + " call in its right operand: Java reads the target before the call, cir/1 after it");
                }
                Map<String, Object> rhs = expr(new TreePath(ep, ct.getExpression()), ctx);
                String op = compoundOp(e.getKind());
                Map<String, Object> lt = type(lhs);
                if (op.equals("<<=") || op.equals(">>=")) {
                    if (!isInt(lt)) {
                        throw new Unsupported(op + " on a " + describe(lt));
                    }
                    return List.of(assign(op, lhs, shiftCount(rhs, lt), pos));
                }
                if (lt.equals(tString())) {
                    if (!op.equals("+=") || !type(rhs).equals(tString())) {
                        throw new Unsupported("+= of a value other than a String to a String");
                    }
                } else if (isFloat(lt)) {
                    if (op.equals("%=")) {
                        throw new Unsupported("%= on a floating-point value: cir/1 % is an integer remainder");
                    }
                    if (!op.equals("+=") && !op.equals("-=") && !op.equals("*=") && !op.equals("/=")) {
                        throw new Unsupported(op + " on a " + describe(lt));
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

    /**
     * Whether an expression calls code that may write what it names: a method or constructor of the
     * package, or the abstract method of a functional interface, which runs a lambda of it.
     */
    private boolean callsCode(TreePath p) {
        boolean[] found = {false};
        new TreePathScanner<Void, Void>() {
            @Override
            public Void visitMethodInvocation(MethodInvocationTree mi, Void x) {
                note(trees.getElement(getCurrentPath()));
                return super.visitMethodInvocation(mi, x);
            }

            @Override
            public Void visitNewClass(NewClassTree nc, Void x) {
                note(trees.getElement(getCurrentPath()));
                return super.visitNewClass(nc, x);
            }

            @Override
            public Void visitLambdaExpression(LambdaExpressionTree l, Void x) {
                return null;
            }

            private void note(Element e) {
                if (e == null || !(e.getEnclosingElement() instanceof TypeElement)) {
                    found[0] = true;
                    return;
                }
                TypeElement owner = (TypeElement) e.getEnclosingElement();
                if (inPackage(owner) || function(owner.getQualifiedName().toString()) != null) {
                    found[0] = true;
                }
            }
        }.scan(p, null);
        return found[0];
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
            case LEFT_SHIFT_ASSIGNMENT:
                return "<<=";
            case RIGHT_SHIFT_ASSIGNMENT:
                return ">>=";
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
        refuseUnassigned(rp, "assigned to field " + target.getSimpleName() + " of another type", ESCAPES);
        TreePath recv = unparen(lp).getLeaf() instanceof MemberSelectTree
            ? unparen(new TreePath(unparen(lp), ((MemberSelectTree) unparen(lp).getLeaf()).getExpression())) : null;
        if (recv != null && !(recv.getLeaf() instanceof IdentifierTree)) {
            throw new Unsupported("an assignment to field " + target.getSimpleName() + ", null until assigned, through a receiver"
                + " that is not a name: Java evaluates the receiver before the value, which the value bound first would not");
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
        boolean arrayElement = k.equals("index") && p.getLeaf() instanceof ArrayAccessTree;
        if (!k.equals("var") && !k.equals("field") && !k.equals("global") && !arrayElement) {
            throw new Unsupported("assignment to something other than a local, a field or an array element");
        }
        return x;
    }

    /**
     * Refuse a String or boxed parameter printed as it is, by concatenation or {@code String.format},
     * in a method that never compares it with {@code null}: Java prints a null one as {@code null},
     * and a cir/1 string or integer has no null. Every other use of a parameter assumes it is not
     * null (README.md, Assumptions). A path parameter only the framework passes ({@link
     * #pathParameter}) is never null.
     */
    private void refusePrintedNullable(TreePath part, Ctx ctx) {
        part = unparen(part);
        Element e = trees.getElement(part);
        if (!(part.getLeaf() instanceof IdentifierTree) || e == null || e.getKind() != ElementKind.PARAMETER) {
            return;
        }
        String q = qname(e.asType());
        if (!q.equals("java.lang.String") && !q.equals("java.lang.Boolean") && !JvmTypes.isIntegerBox(q)
            || pathParameter((VariableElement) e)) {
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
            boolean array = trees.getTypeMirror(ep).getKind() == TypeKind.ARRAY;
            if (!"slice".equals(type(over).get("k")) || !(cls.equals("java.util.List") || cls.equals("java.util.ArrayList") || array)) {
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
                } else if (!exits(end) && !(end instanceof ContinueTree) && !(end instanceof BreakTree) && !last) {
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
        refuseUnassigned(ep, "returned", ESCAPES);
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
                    refuseUnassigned(new TreePath(ep, args.get(0)), "returned in an Optional", ESCAPES);
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
        TreePath np = new TreePath(p, e);
        TypeElement te = (TypeElement) types.asElement(trees.getTypeMirror(np));
        boolean sentinel = errorLeaves.containsKey(te) && nc.getArguments().isEmpty();
        if (project && !sentinel && !formattedLeaves.containsKey(te) && nc.getClassBody() == null && !isProjectClass(te)
            && unchecked(te)) {
            return List.of(raise(np, te, ctx, pos));
        }
        if (!sentinel && !formattedLeaves.containsKey(te) || nc.getClassBody() != null) {
            throw new Unsupported("throw of an exception outside the package's checked-exception hierarchy");
        }
        if (!ctx.fn.throwsErr) {
            throw new Unsupported("throw in a method without an error result");
        }
        List<Object> rs = new ArrayList<>();
        for (Map<String, Object> t : ctx.fn.values) {
            rs.add(zero(t));
        }
        if (sentinel) {
            Map<String, Object> ev = Json.obj();
            ev.put("k", "errvar");
            ev.put("name", te.getSimpleName().toString());
            ev.put("t", tError());
            rs.add(ev);
        } else {
            rs.add(errorf(np, nc, formattedLeaves.get(te), ctx));
        }
        return List.of(ret(rs, pos));
    }

    /** Whether {@code te} is an unchecked exception: a {@code RuntimeException} or an {@code Error}. */
    private boolean unchecked(TypeElement te) {
        for (String base : List.of("java.lang.RuntimeException", "java.lang.Error")) {
            TypeElement b = elements.getTypeElement(base);
            if (b != null && types.isSubtype(te.asType(), b.asType())) {
                return true;
            }
        }
        return false;
    }

    /**
     * {@code throw new X(…)} of a jar's unchecked exception in a method an entry names: {@code raise}
     * of the constructed value. Java leaves the method and every caller up to the framework, which
     * hands the exception to its mapper; {@link #emitFunc} lists the type in {@code raises}. In any
     * other method it is refused: a caller would have to list what it raises, and so its callers.
     */
    private Map<String, Object> raise(TreePath np, TypeElement te, Ctx ctx, String pos) {
        Element m = ctx.fn.inLambda || !(ctx.fn.path.getLeaf() instanceof MethodTree) ? null : trees.getElement(ctx.fn.path);
        if (!(m instanceof ExecutableElement) || !entryMethod((ExecutableElement) m)) {
            throw new Unsupported("throw of " + te.getQualifiedName() + " in a method no entry names: it leaves every caller up"
                + " to the framework, and cir/1 raises only out of a function an entry names");
        }
        if (overridesProjectMethod((ExecutableElement) m)) {
            throw new Unsupported("throw of " + te.getQualifiedName() + " in a method overriding one of the project's: a call"
                + " through the type it overrides would not list what it raises");
        }
        Map<String, Object> x = expr(np, ctx);
        Map<String, Object> t = type(x);
        if (!"extern".equals(t.get("k")) || t.containsKey("targs")) {
            throw new Unsupported("throw of " + te.getQualifiedName() + ", whose value is not of an extern type");
        }
        Map<String, Object> st = Json.obj();
        st.put("k", "raise");
        st.put("x", x);
        st.put("pos", pos);
        return st;
    }

    /** Whether {@code m} overrides a method a class or interface of the project's sources declares. */
    private boolean overridesProjectMethod(ExecutableElement m) {
        TypeElement owner = (TypeElement) m.getEnclosingElement();
        Deque<TypeMirror> todo = new ArrayDeque<>(types.directSupertypes(owner.asType()));
        while (!todo.isEmpty()) {
            TypeElement s = (TypeElement) types.asElement(todo.pop());
            if (s == null) {
                continue;
            }
            if (isProjectClass(s)) {
                for (Element e : s.getEnclosedElements()) {
                    if (e.getKind() == ElementKind.METHOD && elements.overrides(m, (ExecutableElement) e, owner)) {
                        return true;
                    }
                }
            }
            todo.addAll(types.directSupertypes(s.asType()));
        }
        return false;
    }

    /**
     * The extern types the statements raise, sorted by path, then name, each once: the {@code raises}
     * of the function. A refused statement is {@code foreign} and raises nothing here.
     */
    private static List<Object> raisesOf(Object body) {
        TreeMap<String, Object> out = new TreeMap<>();
        Deque<Object> todo = new ArrayDeque<>(List.of(body));
        while (!todo.isEmpty()) {
            Object o = todo.pop();
            if (o instanceof Map) {
                Map<?, ?> m = (Map<?, ?>) o;
                if ("raise".equals(m.get("k"))) {
                    @SuppressWarnings("unchecked")
                    Map<String, Object> t = (Map<String, Object>) ((Map<String, Object>) m.get("x")).get("t");
                    out.put(t.get("path") + "\u0000" + t.get("name"), t);
                }
                todo.addAll(m.values());
            } else if (o instanceof List) {
                todo.addAll((List<?>) o);
            }
        }
        return new ArrayList<>(out.values());
    }

    /**
     * {@code new Leaf(a1, …, an)} of a leaf whose constructor formats its parameters: {@code
     * errorf(<format>, a1, …, an)}, each argument of the constructor's parameter type and printed as
     * {@link #format} prints it.
     */
    private Map<String, Object> errorf(TreePath np, NewClassTree nc, String fmt, Ctx ctx) {
        ExecutableElement ctor = (ExecutableElement) trees.getElement(np);
        List<Character> verbs = formatVerbs(fmt);
        if (verbs.size() != nc.getArguments().size()) {
            throw new Unsupported("an exception format with " + verbs.size() + " conversions and " + nc.getArguments().size()
                + " arguments");
        }
        List<Object> args = new ArrayList<>();
        args.add(lit(quote(fmt), tString()));
        for (int i = 0; i < nc.getArguments().size(); i++) {
            TreePath ap = new TreePath(np, nc.getArguments().get(i));
            refusePrintedNullable(ap, ctx);
            refuseUnassigned(ap, "printed in an exception's message", "Java prints null" + STOPS);
            Map<String, Object> x = coerce(expr(ap, ctx), typeOf(ctor.getParameters().get(i).asType()));
            checkVerb(verbs.get(i), type(x));
            args.add(x);
        }
        return intrinsic("errorf", args, tError());
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
            if (project && structs.contains(te) && fns.get(e) != null) {
                return fns.get(e);
            }
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
        Map<String, Object> folded = wrappedConstant(p);
        if (folded != null) {
            return folded;
        }
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
            case FLOAT_LITERAL:
                if (project) {
                    throw new Unsupported("char and floating-point values are outside the reader's types");
                }
                return floatLit((double) (Float) ((LiteralTree) e).getValue(), "32");
            case DOUBLE_LITERAL:
                if (project) {
                    throw new Unsupported("char and floating-point values are outside the reader's types");
                }
                return floatLit((Double) ((LiteralTree) e).getValue(), "64");
            case CHAR_LITERAL:
                throw new Unsupported("char values are outside the reader's types");
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
                if (project) {
                    throw new Unsupported("shift: Java masks the shift count, cir/1 does not say so");
                }
                return shift(p, ctx);
            case UNSIGNED_RIGHT_SHIFT:
                throw new Unsupported(">>>: an unsigned shift of a signed integer is outside cir/1");
            case UNARY_MINUS:
            case UNARY_PLUS:
            case LOGICAL_COMPLEMENT:
            case BITWISE_COMPLEMENT: {
                refuseUnassigned(new TreePath(p, ((UnaryTree) e).getExpression()), "an operand of a unary operator", "Java unboxes the null" + STOPS);
                Map<String, Object> t = typeOf(trees.getTypeMirror(p));
                Map<String, Object> x = coerce(expr(new TreePath(p, ((UnaryTree) e).getExpression()), ctx), t);
                if (e.getKind() == Tree.Kind.UNARY_MINUS && isFloat(t) && constant(x)) {
                    throw new Unsupported("- of a floating-point constant: a Go constant has no negative zero");
                }
                String op = e.getKind() == Tree.Kind.UNARY_MINUS ? "-"
                    : e.getKind() == Tree.Kind.UNARY_PLUS ? "+" : e.getKind() == Tree.Kind.LOGICAL_COMPLEMENT ? "!" : "^";
                return unop(op, x, t);
            }
            case LAMBDA_EXPRESSION:
                if (project) {
                    throw new Unsupported("lambda: functions as values are outside cir/1");
                }
                return lambda(p, ctx);
            case MEMBER_REFERENCE:
                if (project) {
                    throw new Unsupported("method reference: functions as values are outside cir/1");
                }
                return methodRef(p, ctx);
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
            case ARRAY_ACCESS: {
                if (project) {
                    throw new Unsupported("arrays are outside the reader's types");
                }
                ArrayAccessTree aa = (ArrayAccessTree) e;
                Map<String, Object> x = expr(new TreePath(p, aa.getExpression()), ctx);
                Map<String, Object> i = coerce(expr(new TreePath(p, aa.getIndex()), ctx), tInt("32"));
                return index(x, i, typeOf(trees.getTypeMirror(p)));
            }
            case NEW_ARRAY: {
                NewArrayTree na = (NewArrayTree) e;
                if (project || !na.getDimensions().isEmpty() || na.getInitializers() == null) {
                    throw new Unsupported("an array of a given length: cir/1 makes only an empty slice; write its elements"
                        + " out, {a, b}");
                }
                Map<String, Object> st = typeOf(trees.getTypeMirror(p));
                List<Object> args = new ArrayList<>();
                Map<String, Object> mk = kind("make");
                mk.put("t", st);
                args.add(mk);
                for (ExpressionTree x : na.getInitializers()) {
                    TreePath xp = new TreePath(p, x);
                    refuseUnassigned(xp, "an element of an array", ESCAPES);
                    args.add(coerce(expr(xp, ctx), sub(st, "elem")));
                }
                return args.size() == 1 ? mk : intrinsic("append", args, st);
            }
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
                return fieldRef((VariableElement) el, null, ctx, ctx == null || ctx.fn == null ? null : ctx.fn.cls);
            default:
                throw new Unsupported(el.getKind().toString().toLowerCase(Locale.ROOT) + " " + id.getName() + " as a value");
        }
    }

    private Map<String, Object> select(TreePath p, Ctx ctx) {
        MemberSelectTree ms = (MemberSelectTree) p.getLeaf();
        if (ms.getIdentifier().contentEquals("class")) {
            throw new Unsupported("class literal: reflection is outside cir/1");
        }
        TreePath xp = new TreePath(p, ms.getExpression());
        if (!project && ms.getIdentifier().contentEquals("length") && trees.getTypeMirror(xp).getKind() == TypeKind.ARRAY) {
            return size(expr(xp, ctx));
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
        TypeMirror xt = trees.getTypeMirror(xp);
        TypeElement holder = xt != null && xt.getKind() == TypeKind.DECLARED ? (TypeElement) ((DeclaredType) xt).asElement() : null;
        return fieldRef((VariableElement) el, expr(new TreePath(p, ms.getExpression()), ctx), ctx, holder);
    }

    private Map<String, Object> fieldRef(VariableElement el, Map<String, Object> recv, Ctx ctx) {
        return fieldRef(el, recv, ctx, null);
    }

    /**
     * A field read or written; {@code holder} is the class of the object it is reached through, the
     * receiver's static type or the class of {@code this}, or null when not known.
     */
    private Map<String, Object> fieldRef(VariableElement el, Map<String, Object> recv, Ctx ctx, TypeElement holder) {
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
            if (vars.containsKey(el)) {
                if (ctx != null && ctx.used.contains(vars.get(el))) {
                    throw new Unsupported("static field " + owner.getSimpleName() + "." + name + " where a local or a parameter"
                        + " is named " + vars.get(el) + ": a module-level variable and a local of one name are one name in"
                        + " the written function");
                }
                Map<String, Object> g = kind("global");
                g.put("name", vars.get(el));
                g.put("t", typeOf(el.asType()));
                return g;
            }
            String on = owner.getQualifiedName().toString();
            if ((on.equals("java.lang.Integer") || on.equals("java.lang.Long")) && el.getConstantValue() != null) {
                return constLit(el.getConstantValue(), typeOf(el.asType()));
            }
            throw new Unsupported("static field " + owner.getSimpleName() + "." + name);
        }
        TypeElement through = holder != null ? holder : owner;
        if (project && (inOtherPackage(owner) || inOtherPackage(through))) {
            throw new Unsupported("field " + name + " of " + through.getQualifiedName() + ", a class of another package: its"
                + " object is an extern here, whose fields are not part of the module, and Java reads and writes the one"
                + " object the caller holds");
        }
        boolean inherited = project && el.equals(panacheId()) && holder != null && structs.contains(holder)
            && extendsPanacheEntity(holder);
        if (!structs.contains(owner) && !inherited) {
            throw new Unsupported("field of " + owner.getQualifiedName() + ", which is not read");
        }
        if (project) {
            String accessor = managedAccessor(holder != null ? holder : owner, owner, el, ctx);
            if (accessor != null) {
                throw new Unsupported("field " + name + " of the managed class " + (holder != null ? holder : owner).getSimpleName()
                    + ": Quarkus (Panache or Hibernate ORM) makes the access a call of " + accessor + ", which the class declares");
            }
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
     * The getter or setter of field {@code el} that a managed class declares, when Quarkus turns the
     * access into a call of it, which runs that method's code; null for any other access.
     *
     * <p>Quarkus 3 ({@code PanacheHibernateCommonResourceProcessor}, Panache or plain Hibernate ORM)
     * rewrites every read and write of a public, non-static, non-{@code @Transient} field of a
     * managed class ({@code @Entity}, {@code @MappedSuperclass}, {@code @Embeddable}, not a record)
     * into a call of its getter or setter, in every class, the managed class's own methods included,
     * except in a constructor of the class the access goes through or of a managed subclass of it,
     * and in that class's getter or setter of the field itself. A class without that method gets a
     * generated one, which only reads or writes the field. The method is matched by name and
     * descriptor: {@code is} for a {@code boolean} field, {@code get} for any other, without
     * parameters and of the field's type; {@code set} with one parameter of the field's type and no
     * result; each before the field's name with its first letter upper-cased, or unchanged when its
     * second letter is upper case ({@code xCoord} gives {@code getxCoord}). A class extending a
     * Panache entity base counts as managed whether it is annotated or not.
     */
    private String managedAccessor(TypeElement holder, TypeElement owner, VariableElement el, Ctx ctx) {
        if (!el.getModifiers().contains(Modifier.PUBLIC) || el.getModifiers().contains(Modifier.STATIC)
            || annotation(el, "jakarta.persistence.Transient") != null) {
            return null;
        }
        List<TypeElement> chain = new ArrayList<>();
        for (TypeElement c = holder; ; c = (TypeElement) types.asElement(c.getSuperclass())) {
            if (!managed(c)) {
                return null;
            }
            chain.add(c);
            if (c.equals(owner)) {
                break;
            }
            if (c.getSuperclass().getKind() != TypeKind.DECLARED) {
                return null;
            }
        }
        String name = el.getSimpleName().toString();
        String cap = name.length() > 1 && Character.isUpperCase(name.charAt(1)) ? name
            : Character.toUpperCase(name.charAt(0)) + name.substring(1);
        String getter = (el.asType().getKind() == TypeKind.BOOLEAN ? "is" : "get") + cap;
        String setter = "set" + cap;
        TypeMirror ft = types.erasure(el.asType());
        Fn fn = ctx == null ? null : ctx.fn;
        Element self = null;
        if (fn != null && !fn.inLambda && fn.cls != null) {
            if (fn.isCtor && inManagedHierarchy(fn.cls, holder)) {
                return null;
            }
            Element m = fn.path != null && fn.path.getLeaf() instanceof MethodTree ? trees.getElement(fn.path) : null;
            if (m instanceof ExecutableElement && fn.cls.equals(holder) && holder.equals(owner)
                && isAccessor((ExecutableElement) m, getter, setter, ft)) {
                // Inside the getter its read is direct and a write calls the setter; inside the setter
                // the other way round. Whether this access reads or writes is not known here, so only
                // the other accessor, declared, makes it a call of code.
                self = m;
            }
        }
        for (TypeElement c : chain) {
            for (Element m : c.getEnclosedElements()) {
                if (m.getKind() == ElementKind.METHOD && !m.equals(self) && isAccessor((ExecutableElement) m, getter, setter, ft)) {
                    return c.getSimpleName() + "." + m.getSimpleName();
                }
            }
        }
        return null;
    }

    /** Whether {@code m} is the getter {@code getter} or the setter {@code setter} of a field of erased type {@code ft}. */
    private boolean isAccessor(ExecutableElement m, String getter, String setter, TypeMirror ft) {
        String n = m.getSimpleName().toString();
        if (n.equals(getter)) {
            return m.getParameters().isEmpty() && types.isSameType(types.erasure(m.getReturnType()), ft);
        }
        return n.equals(setter) && m.getParameters().size() == 1 && m.getReturnType().getKind() == TypeKind.VOID
            && types.isSameType(types.erasure(m.getParameters().get(0).asType()), ft);
    }

    /** Whether Quarkus manages {@code c}: an {@code @Entity}, {@code @MappedSuperclass} or {@code @Embeddable} class, or a Panache entity. */
    private boolean managed(TypeElement c) {
        if (c.getKind() != ElementKind.CLASS) {
            return false;
        }
        for (String a : MANAGED) {
            if (annotation(c, a) != null) {
                return true;
            }
        }
        return panacheEntity(c);
    }

    /**
     * Whether {@code cls} is {@code target} or reaches it through managed superclasses: a constructor
     * of such a class accesses the field directly.
     */
    private boolean inManagedHierarchy(TypeElement cls, TypeElement target) {
        for (TypeElement c = cls; ; c = (TypeElement) types.asElement(c.getSuperclass())) {
            if (c.equals(target)) {
                return true;
            }
            if (!managed(c) || c.getSuperclass().getKind() != TypeKind.DECLARED) {
                return false;
            }
        }
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

    /** Why a read that hands the null on (returned, passed, bound, assigned) is refused. */
    private static final String ESCAPES = "Java hands the null on" + STOPS;

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
        ExecutableElement m = (ExecutableElement) trees.getElement(p);
        if (project && mayRaise(m)) {
            throw new Unsupported("call of " + m.getEnclosingElement().getSimpleName() + "." + m.getSimpleName() + ", which raises:"
                + " a raise leaves every caller up to the framework, and only a function an entry names raises here");
        }
        if (!mi.getTypeArguments().isEmpty() && !genericMethods.contains(m)) {
            throw new Unsupported("explicit type arguments: generics are outside cir/1");
        }
        TypeElement owner = (TypeElement) m.getEnclosingElement();
        String on = owner.getQualifiedName().toString();
        String name = m.getSimpleName().toString();
        if (project && on.equals("java.lang.Class") && classNameOf(p) != null) {
            return lit(quote(classNameOf(p)), tString());
        }
        if (on.equals("java.lang.Class") || on.startsWith("java.lang.reflect.") || name.equals("getClass")) {
            throw new Unsupported("reflection: " + owner.getSimpleName() + "." + name + " is outside cir/1");
        }
        if (on.startsWith("java.util.stream.") || (on.startsWith("java.util.") && (name.equals("stream") || name.equals("parallelStream")))) {
            throw new Unsupported("stream: " + owner.getSimpleName() + "." + name + " is outside cir/1");
        }
        if (on.startsWith("java.util.function.") && (project || function(on) == null || !name.equals(function(on)[1]))) {
            throw new Unsupported("call through a functional interface other than the abstract method of one the reader"
                + " reads as a function type");
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
            Map<String, Map<String, Object>> bound = typeArguments(p, mi, m, fn);
            List<Object> args = args(p, mi.getArguments(), substitute(fn.params, bound), fn.variadic, ctx);
            Map<String, Object> c = Json.obj();
            if (fn.isStatic) {
                c.put("k", "call");
                c.put("name", fn.name);
                if (!fn.tparams.isEmpty()) {
                    List<Object> targs = new ArrayList<>();
                    for (String tp : fn.tparams) {
                        targs.add(bound.get(tp));
                    }
                    c.put("targs", targs);
                }
            } else {
                c.put("k", "method");
                c.put("x", recvPath == null ? thisRef(ctx) : expr(recvPath, ctx));
                c.put("name", fn.name);
            }
            c.put("args", args);
            c.put("t", substitute(List.of(resultType(fn)), bound).get(0));
            return c;
        }
        TypeElement io = owner.getKind() == ElementKind.INTERFACE ? owner : null;
        if (io != null && impls.containsKey(io)) {
            return ifaceCall(p, mi, m, io, recvPath, ctx);
        }
        String[] sam = function(on);
        if (sam != null && name.equals(sam[1]) && recvPath != null) {
            Map<String, Object> f = expr(recvPath, ctx);
            Map<String, Object> ft = type(f);
            if (!"func".equals(ft.get("k"))) {
                throw new Unsupported(owner.getSimpleName() + "." + name + " on a " + describe(ft));
            }
            @SuppressWarnings("unchecked")
            List<Map<String, Object>> fps = (List<Map<String, Object>>) ft.get("params");
            @SuppressWarnings("unchecked")
            List<Map<String, Object>> frs = (List<Map<String, Object>>) ft.get("results");
            Map<String, Object> c = kind("call_value");
            c.put("f", f);
            c.put("args", args(p, mi.getArguments(), fps, false, ctx));
            c.put("t", frs.isEmpty() ? tUnit() : frs.get(0));
            return c;
        }
        if (!project && isSystemOut(recvPath) && on.equals("java.io.PrintStream") && name.equals("println") && mi.getArguments().size() <= 1) {
            return println(p, mi, ctx);
        }
        if (inPackage(owner) && !(project && owner.getKind() == ElementKind.INTERFACE && !implOf.containsKey(owner))) {
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
            if (project) {
                return externCall(p, mi, m, owner, recvPath, ctx);
            }
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
        if (project) {
            return externCall(p, mi, m, owner, recvPath, ctx);
        }
        throw new Unsupported(owner.getSimpleName() + "." + name + " is outside the reader's library table");
    }

    /**
     * A call of a method of a class outside the module, from a jar or another package of the
     * project, as an extern with its declared signature: {@code extern} for a static method,
     * {@code extern_method} on an object. Overloads are not told apart: externs are keyed by name.
     */
    private Map<String, Object> externCall(TreePath p, MethodInvocationTree mi, ExecutableElement m, TypeElement owner,
                                           TreePath recvPath, Ctx ctx) {
        boolean isStatic = m.getModifiers().contains(Modifier.STATIC);
        String name = m.getSimpleName().toString();
        Fn foreign = null;
        if (inOtherPackage(owner)) {
            if (owner.getKind() == ElementKind.INTERFACE) {
                throw new Unsupported("call of " + owner.getQualifiedName() + "." + name + " through an interface of another"
                    + " package: the reader builds a call of another package only from a function its module declares");
            }
            foreign = declaredFn(m);
            if (foreign == null) {
                throw new Unsupported("call of " + owner.getQualifiedName() + "." + name + ", which its package does not read:"
                    + " the module of the package declares no function for it");
            }
            String why = foreignCallShape(foreign);
            if (why != null) {
                throw new Unsupported("call of " + owner.getQualifiedName() + "." + name + " of another package: " + why);
            }
        }
        Map<String, Object> recv = null;
        if (!isStatic) {
            recv = recvPath == null ? thisRef(ctx) : expr(recvPath, ctx);
            Map<String, Object> rt = type(recv);
            String rk = (String) rt.get("k");
            boolean own = "ptr".equals(rk) && "named".equals(sub(rt, "elem").get("k"));
            if (!"extern".equals(rk) && !own) {
                throw new Unsupported(owner.getSimpleName() + "." + name + " on a " + describe(rt)
                    + ": the value is not an object of a class outside the module");
            }
        }
        List<Map<String, Object>> sigParams = new ArrayList<>();
        List<Map<String, Object>> sigResults = new ArrayList<>();
        if (foreign != null) {
            sigParams.addAll(foreignTypes(foreign.params, packageOf(owner)));
            sigResults.addAll(foreignTypes(foreign.results(), packageOf(owner)));
        } else {
            for (VariableElement v : m.getParameters()) {
                sigParams.add(signatureType(v.asType()));
            }
            if (m.getReturnType().getKind() != TypeKind.VOID) {
                sigResults.add(signatureType(m.getReturnType()));
            }
        }
        boolean variadic = m.isVarArgs();
        Map<String, Object> declared = Json.obj();
        String path = isStatic ? staticPath(owner) : externPath(owner);
        String typeName = isStatic ? "" : externName(owner);
        declared.put("path", path);
        declared.put("name", name);
        if (!isStatic) {
            declared.put("recv", externType(owner, List.of()));
        }
        Map<String, Object> entity = isStatic ? panacheStaticEntity(owner, name, recvPath, ctx) : null;
        if (entity != null) {
            Map<String, Object> tp = Json.obj();
            tp.put("name", "E");
            tp.put("constraint", kind("any"));
            declared.put("tparams", new ArrayList<Object>(List.of(tp)));
        }
        declared.put("params", new ArrayList<Object>(sigParams));
        declared.put("results", new ArrayList<Object>(sigResults));
        if (variadic) {
            declared.put("variadic", Boolean.TRUE);
        }
        List<? extends ExpressionTree> as = mi.getArguments();
        List<Object> args = new ArrayList<>();
        for (int i = 0; i < as.size(); i++) {
            TreePath ap = new TreePath(p, as.get(i));
            refuseUnassigned(ap, "an argument of a call", ESCAPES);
            Map<String, Object> want = i < sigParams.size() - (variadic ? 1 : 0) ? sigParams.get(i)
                : variadic ? sub(sigParams.get(sigParams.size() - 1), "elem") : null;
            args.add(externArg(expr(ap, ctx), want));
        }
        Map<String, Object> t = m.getReturnType().getKind() == TypeKind.VOID ? tUnit()
            : foreign != null ? sigResults.get(0) : signatureType(trees.getTypeMirror(p));
        registerExtern(path, typeName, name, declared);
        Map<String, Object> c = Json.obj();
        if (isStatic) {
            c.put("k", "extern");
            c.put("path", path);
            c.put("name", name);
            if (entity != null) {
                c.put("targs", new ArrayList<Object>(List.of(entity)));
            }
        } else {
            c.put("k", "extern_method");
            c.put("path", path);
            c.put("type", typeName);
            c.put("name", name);
            c.put("recv", recv);
        }
        c.put("args", args);
        c.put("t", t);
        return c;
    }

    /**
     * The entity a static method of a Panache entity base is called on, as the type argument of the
     * extern's one type parameter {@code E}, or null for any other static method. Panache gives each
     * entity its own copy of the method, run on that entity's table: {@code Fruit.deleteAll()} and
     * {@code Other.deleteAll()} are one Java method and two operations. The entity is the class javac
     * calls the method on: the class the call names; for an unqualified call, the calling class when
     * it inherits the method, else the class a static import names. javac calls the base itself for
     * a call naming the base and for an unqualified call in a class nested in an entity; Panache gives
     * the base no copy and its own method only throws, so such a call is refused, as is one where
     * javac's class is not known (a static field's initializer).
     */
    private Map<String, Object> panacheStaticEntity(TypeElement owner, String name, TreePath recvPath, Ctx ctx) {
        if (!project || !PANACHE_ENTITY_BASES.contains(owner.getQualifiedName().toString())) {
            return null;
        }
        TypeElement entity = null;
        if (recvPath != null && trees.getElement(recvPath) instanceof TypeElement) {
            entity = (TypeElement) trees.getElement(recvPath);
        } else if (recvPath == null && ctx != null && ctx.fn != null && ctx.fn.cls != null) {
            if (types.isSubtype(types.erasure(ctx.fn.cls.asType()), types.erasure(owner.asType()))) {
                entity = ctx.fn.cls;
            } else {
                boolean outer = false;
                for (Element e = ctx.fn.cls.getEnclosingElement(); e instanceof TypeElement; e = e.getEnclosingElement()) {
                    outer |= types.isSubtype(types.erasure(e.asType()), types.erasure(owner.asType()));
                }
                entity = outer ? owner : staticImport(ctx.cu, name, owner);
            }
        }
        if (entity == null || entity.equals(owner)) {
            throw new Unsupported("a Panache static method " + name + " that javac calls on " + owner.getSimpleName()
                + " itself, or on a class not known here: Panache runs a copy of it for each entity, and the base's own"
                + " method only throws");
        }
        if (structs.contains(entity)) {
            return tNamed(entity.getSimpleName().toString());
        }
        if (inPackage(entity) || inOtherPackage(entity) && !declared(entity)) {
            throw new Unsupported("a Panache static method called on " + entity.getSimpleName() + ", which is not read");
        }
        return externType(entity, List.of());
    }

    /**
     * The class a static import of {@code cu} brings the method {@code name} in from, a subclass of
     * {@code base}: a single-static-import of it, else an on-demand one; null when none does.
     */
    private TypeElement staticImport(CompilationUnitTree cu, String name, TypeElement base) {
        TypeElement onDemand = null;
        for (ImportTree it : cu.getImports()) {
            if (!it.isStatic() || !(it.getQualifiedIdentifier() instanceof MemberSelectTree)) {
                continue;
            }
            MemberSelectTree q = (MemberSelectTree) it.getQualifiedIdentifier();
            TypeElement from = elements.getTypeElement(q.getExpression().toString());
            if (from == null || !types.isSubtype(types.erasure(from.asType()), types.erasure(base.asType()))) {
                continue;
            }
            if (q.getIdentifier().contentEquals(name)) {
                return from;
            }
            if (q.getIdentifier().contentEquals("*") && onDemand == null) {
                onDemand = from;
            }
        }
        return onDemand;
    }

    /** The path of a static extern: the package of another package's class, else the class. */
    private String staticPath(TypeElement owner) {
        return inOtherPackage(owner) ? packageOf(owner) : owner.getQualifiedName().toString();
    }

    /** An argument for a parameter of the declared type {@code want}: a subtype keeps its own type. */
    private Map<String, Object> externArg(Map<String, Object> e, Map<String, Object> want) {
        if (want == null) {
            return e;
        }
        String k = (String) want.get("k");
        if ("int".equals(k) || "bool".equals(k) || "string".equals(k)) {
            return coerce(e, want);
        }
        return e;
    }

    /** Declare an extern; a second declaration under the same key that differs is an overload, refused. */
    private void registerExtern(String path, String type, String name, Map<String, Object> declared) {
        String key = externKey(path, type, name);
        Map<String, Object> have = externs.get(key);
        if (have != null && !Json.write(have).equals(Json.write(declared))) {
            throw new Unsupported("an overload of " + name + " in " + path + ": externs are keyed by name, and the declaration"
                + " already read has other parameters");
        }
        externs.put(key, declared);
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
        List<Character> verbs = formatVerbs(f);
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
            checkVerb(verbs.get(i - 1), type(x));
            args.add(x);
        }
        return intrinsic("sprintf", args, tString());
    }

    /** The conversions of a literal format, {@code s} or {@code d} each; any other is refused. */
    private static List<Character> formatVerbs(String f) {
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
        return verbs;
    }

    /** Refuse a value a conversion prints otherwise than {@code sprintf}: only %s of a String and %d of an integer. */
    private static void checkVerb(char verb, Map<String, Object> t) {
        boolean fits = verb == 's' ? t.equals(tString()) : isInt(t);
        if (!fits) {
            throw new Unsupported("String.format %" + verb + " of a " + describe(t)
                + ": only %s of a String and %d of an integer print alike");
        }
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
            Element called = trees.getElement(p);
            if (project && fns.get(called) != null) {
                fn = fns.get(called);
            }
            if (fn == null) {
                throw new Unsupported("a call of a constructor of " + te.getSimpleName() + " that is not read");
            }
            boolean synthesized = fns.get(called) == null && called != null && generated(called) && fn.params.isEmpty();
            if (fns.get(called) != fn && !synthesized || nc.getArguments().size() != fn.params.size()) {
                throw new Unsupported("a call of a constructor of " + te.getSimpleName() + " that is not read: the class"
                    + " reads one constructor");
            }
            if (fn.throwsErr && !mayThrow) {
                throw new Unsupported("a constructor that throws, inside an expression");
            }
            if (fn.foreignThrows != null) {
                throw new Unsupported("constructor of " + te.getSimpleName() + ", which throws " + fn.foreignThrows);
            }
            Map<String, Object> c = Json.obj();
            c.put("k", "call");
            c.put("name", fn.name);
            c.put("args", args(p, nc.getArguments(), fn.params, fn.variadic, ctx));
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
        if (errorLeaves.containsKey(te) || formattedLeaves.containsKey(te)) {
            throw new Unsupported("an exception created other than by throw");
        }
        if (project && !inPackage(te) && trees.getElement(p) instanceof ExecutableElement) {
            return externNew(p, nc, te, (ExecutableElement) trees.getElement(p), tm, ctx);
        }
        throw new Unsupported("new " + te.getSimpleName() + " is outside the reader's library table");
    }

    /**
     * {@code new C(…)} of a class outside the module as an extern function: {@code <init>} of the
     * class for a jar's, {@code New<C>} of the package for a class of another package.
     */
    private Map<String, Object> externNew(TreePath p, NewClassTree nc, TypeElement te, ExecutableElement ce, TypeMirror tm, Ctx ctx) {
        Fn foreign = null;
        if (inOtherPackage(te)) {
            foreign = declaredFn(ce);
            if (foreign == null) {
                throw new Unsupported("a call of a constructor of " + te.getQualifiedName() + " that its package does not read:"
                    + " the module of the package declares no function for it");
            }
            String why = foreignCallShape(foreign);
            if (why != null) {
                throw new Unsupported("a call of a constructor of " + te.getQualifiedName() + " of another package: " + why);
            }
        }
        List<Map<String, Object>> sigParams = new ArrayList<>();
        List<Map<String, Object>> sigResults = new ArrayList<>();
        if (foreign != null) {
            sigParams.addAll(foreignTypes(foreign.params, packageOf(te)));
            sigResults.addAll(foreignTypes(foreign.results(), packageOf(te)));
        } else {
            for (VariableElement v : ce.getParameters()) {
                sigParams.add(signatureType(v.asType()));
            }
            sigResults.add(externType(te, List.of()));
        }
        boolean variadic = ce.isVarArgs();
        String path = staticPath(te);
        String name = foreign != null ? foreign.name : "<init>";
        Map<String, Object> declared = Json.obj();
        declared.put("path", path);
        declared.put("name", name);
        declared.put("params", new ArrayList<Object>(sigParams));
        declared.put("results", new ArrayList<Object>(sigResults));
        if (variadic) {
            declared.put("variadic", Boolean.TRUE);
        }
        List<Object> args = new ArrayList<>();
        List<? extends ExpressionTree> as = nc.getArguments();
        for (int i = 0; i < as.size(); i++) {
            TreePath ap = new TreePath(p, as.get(i));
            refuseUnassigned(ap, "an argument of a constructor", ESCAPES);
            Map<String, Object> want = i < sigParams.size() - (variadic ? 1 : 0) ? sigParams.get(i)
                : variadic ? sub(sigParams.get(sigParams.size() - 1), "elem") : null;
            args.add(externArg(expr(ap, ctx), want));
        }
        registerExtern(path, "", name, declared);
        Map<String, Object> c = Json.obj();
        c.put("k", "extern");
        c.put("path", path);
        c.put("name", name);
        c.put("args", args);
        c.put("t", foreign != null ? sigResults.get(0) : signatureType(tm));
        return c;
    }

    /**
     * The arguments of a call. For a variable arity method, Java passes an array as the last
     * argument itself, which is a {@code spread} of the slice; any other arguments for the last
     * parameter are its elements, each of the slice's element type.
     */
    private List<Object> args(TreePath p, List<? extends ExpressionTree> as, List<Map<String, Object>> params, boolean variadic,
                              Ctx ctx) {
        List<Object> out = new ArrayList<>();
        int fixed = variadic ? params.size() - 1 : params.size();
        for (int i = 0; i < as.size(); i++) {
            TreePath ap = new TreePath(p, as.get(i));
            refuseUnassigned(ap, "an argument of a call", ESCAPES);
            if (i < fixed) {
                out.add(coerce(expr(ap, ctx), params.get(i)));
                continue;
            }
            Map<String, Object> slice = params.get(params.size() - 1);
            if (as.size() == params.size() && trees.getTypeMirror(ap).getKind() == TypeKind.ARRAY) {
                Map<String, Object> s = kind("spread");
                s.put("x", coerce(expr(ap, ctx), slice));
                s.put("t", slice);
                out.add(s);
            } else {
                out.add(coerce(expr(ap, ctx), sub(slice, "elem")));
            }
        }
        return out;
    }

    /**
     * The type arguments of a call of a generic method: written out, or as javac infers them from
     * the method's type at the call, each the type of an argument whose parameter is that type
     * variable, else of the result.
     */
    private Map<String, Map<String, Object>> typeArguments(TreePath p, MethodInvocationTree mi, ExecutableElement m, Fn fn) {
        Map<String, Map<String, Object>> bound = new HashMap<>();
        if (fn.tparams.isEmpty()) {
            return bound;
        }
        List<? extends javax.lang.model.element.TypeParameterElement> tps = m.getTypeParameters();
        if (!mi.getTypeArguments().isEmpty()) {
            for (int i = 0; i < tps.size(); i++) {
                bound.put(fn.tparams.get(i), typeOf(trees.getTypeMirror(new TreePath(p, mi.getTypeArguments().get(i)))));
            }
            return bound;
        }
        TypeMirror at = trees.getTypeMirror(new TreePath(p, mi.getMethodSelect()));
        if (!(at instanceof javax.lang.model.type.ExecutableType)) {
            throw new Unsupported("a call of generic method " + fn.name + " whose type arguments javac does not give");
        }
        javax.lang.model.type.ExecutableType inst = (javax.lang.model.type.ExecutableType) at;
        for (int i = 0; i < tps.size(); i++) {
            TypeMirror tv = tps.get(i).asType();
            TypeMirror found = null;
            for (int j = 0; j < m.getParameters().size() && found == null; j++) {
                if (types.isSameType(m.getParameters().get(j).asType(), tv)) {
                    found = inst.getParameterTypes().get(j);
                }
            }
            if (found == null && types.isSameType(m.getReturnType(), tv)) {
                found = inst.getReturnType();
            }
            if (found == null) {
                throw new Unsupported("a call of generic method " + fn.name + " whose type argument for "
                    + tps.get(i).getSimpleName() + " no parameter or result names: write it out");
            }
            bound.put(fn.tparams.get(i), typeOf(found));
        }
        return bound;
    }

    /** {@code ts} with each type parameter {@code bound} names replaced by its type argument. */
    @SuppressWarnings("unchecked")
    private static List<Map<String, Object>> substitute(List<Map<String, Object>> ts, Map<String, Map<String, Object>> bound) {
        if (bound.isEmpty()) {
            return ts;
        }
        List<Map<String, Object>> out = new ArrayList<>();
        for (Map<String, Object> t : ts) {
            if ("param".equals(t.get("k")) && bound.containsKey(t.get("name"))) {
                out.add(bound.get(t.get("name")));
                continue;
            }
            Map<String, Object> c = Json.obj();
            for (Map.Entry<String, Object> e : t.entrySet()) {
                Object v = e.getValue();
                if (v instanceof Map) {
                    v = substitute(List.of((Map<String, Object>) v), bound).get(0);
                } else if (v instanceof List) {
                    List<Object> l = new ArrayList<>();
                    for (Object o : (List<?>) v) {
                        l.add(o instanceof Map ? substitute(List.of((Map<String, Object>) o), bound).get(0) : o);
                    }
                    v = l;
                }
                c.put(e.getKey(), v);
            }
            out.add(c);
        }
        return out;
    }

    /** A call of a method of an interface with several implementations: the method of the object it holds runs. */
    private Map<String, Object> ifaceCall(TreePath p, MethodInvocationTree mi, ExecutableElement m, TypeElement ie, TreePath recvPath,
                                          Ctx ctx) {
        if (recvPath == null) {
            throw new Unsupported("a call of interface method " + m.getSimpleName() + " without a receiver");
        }
        Map<String, Object> sig = null;
        for (Map<String, Object> x : ifaceMethods.get(ie)) {
            if (ifaceMethodName(m.getSimpleName().toString()).equals(x.get("name"))) {
                sig = x;
            }
        }
        if (sig == null) {
            throw new Unsupported("method " + m.getSimpleName() + " is not one of interface " + ie.getSimpleName());
        }
        Map<String, Object> recv = coerce(expr(recvPath, ctx), tNamed(ie.getSimpleName().toString()));
        @SuppressWarnings("unchecked")
        List<Map<String, Object>> ps = (List<Map<String, Object>>) sig.get("params");
        @SuppressWarnings("unchecked")
        List<Map<String, Object>> rs = (List<Map<String, Object>>) sig.get("results");
        Map<String, Object> c = kind("iface_call");
        c.put("iface", ie.getSimpleName().toString());
        c.put("name", ifaceMethodName(m.getSimpleName().toString()));
        c.put("recv", recv);
        c.put("args", args(p, mi.getArguments(), ps, false, ctx));
        c.put("t", rs.isEmpty() ? tUnit() : rs.get(0));
        return c;
    }

    /** The function type a lambda or method reference is converted to, or refused. */
    private Map<String, Object> functionTarget(TreePath p) {
        Map<String, Object> ft = typeOf(trees.getTypeMirror(p));
        if (!"func".equals(ft.get("k"))) {
            throw new Unsupported("a function value of " + describe(ft) + ": only the functional interfaces the reader reads as"
                + " function types");
        }
        return ft;
    }

    /**
     * {@code Class::method} of a static method the module reads, or {@code x::method} of an
     * instance method bound to the value of {@code x}, which Java evaluates once, where the
     * reference stands, as cir/1 does; the method's signature must be the function type's, so Java
     * neither boxes nor widens through it.
     */
    private Map<String, Object> methodRef(TreePath p, Ctx ctx) {
        MemberReferenceTree mr = (MemberReferenceTree) p.getLeaf();
        Map<String, Object> ft = functionTarget(p);
        if (mr.getMode() != MemberReferenceTree.ReferenceMode.INVOKE) {
            throw new Unsupported("a constructor reference: cir/1 has no function value of a constructor");
        }
        Element e = trees.getElement(p);
        Fn fn = e == null ? null : fns.get(e);
        if (fn == null || fn.isCtor) {
            throw new Unsupported("a reference to a method the module does not read");
        }
        if (fn.throwsErr || fn.optional || fn.variadic || !fn.tparams.isEmpty()) {
            throw new Unsupported("a reference to a method that throws, returns an Optional, takes variable arity or is generic");
        }
        if (!new ArrayList<Object>(fn.params).equals(ft.get("params")) || !new ArrayList<Object>(fn.results()).equals(ft.get("results"))) {
            throw new Unsupported("a reference to " + fn.key() + ", whose signature is not the function type's: Java converts"
                + " through it");
        }
        TreePath qp = new TreePath(p, mr.getQualifierExpression());
        Map<String, Object> r = kind("func_ref");
        r.put("name", fn.name);
        if (fn.isStatic) {
            if (!namesAType(qp)) {
                throw new Unsupported("a static method referenced through an expression, which Java evaluates and then ignores");
            }
        } else {
            if (namesAType(qp)) {
                throw new Unsupported("a reference to an instance method through its class, which takes the object as its first"
                    + " argument");
            }
            refuseUnassigned(qp, "the receiver of a method reference", ESCAPES);
            r.put("recv", expr(qp, ctx));
        }
        r.put("t", ft);
        return r;
    }

    /**
     * A lambda: its parameters and results are those of the function type, its body read as a
     * function's, returning the function's result. Java captures only effectively final locals,
     * so capturing by value and by reference are alike; {@code captures} names each local,
     * parameter or {@code this} of the enclosing function it uses.
     */
    private Map<String, Object> lambda(TreePath p, Ctx ctx) {
        LambdaExpressionTree lt = (LambdaExpressionTree) p.getLeaf();
        Map<String, Object> ft = functionTarget(p);
        @SuppressWarnings("unchecked")
        List<Map<String, Object>> fps = (List<Map<String, Object>>) ft.get("params");
        @SuppressWarnings("unchecked")
        List<Map<String, Object>> frs = (List<Map<String, Object>>) ft.get("results");
        Fn lf = new Fn();
        lf.name = ctx.fn.name;
        lf.isStatic = ctx.fn.isStatic;
        lf.cls = ctx.fn.cls;
        lf.path = ctx.fn.path;
        lf.inLambda = true;
        lf.values.addAll(frs);
        Ctx lc = new Ctx(lf, ctx.cu, ctx.hasThis);
        lc.used.addAll(ctx.used);
        List<Object> params = new ArrayList<>();
        Set<Element> own = new HashSet<>();
        for (int i = 0; i < lt.getParameters().size(); i++) {
            VariableTree v = lt.getParameters().get(i);
            Element ve = trees.getElement(new TreePath(p, v));
            own.add(ve);
            if (!typeOf(ve.asType()).equals(fps.get(i))) {
                throw new Unsupported("a lambda parameter of a type other than the function type's");
            }
            Map<String, Object> pm = Json.obj();
            pm.put("name", v.getName().toString());
            pm.put("t", fps.get(i));
            params.add(pm);
        }
        TreePath bp = new TreePath(p, lt.getBody());
        List<Object> body = new ArrayList<>();
        if (lt.getBodyKind() == LambdaExpressionTree.BodyKind.EXPRESSION) {
            if (frs.isEmpty()) {
                body.addAll(stmtOf(bp, lc));
            } else {
                refuseUnassigned(bp, "returned", ESCAPES);
                body.add(ret(List.of(coerce(expr(bp, lc), frs.get(0))), pos(ctx.cu, lt.getBody())));
            }
        } else {
            BlockTree bt = (BlockTree) lt.getBody();
            for (StatementTree s : bt.getStatements()) {
                List<Object> out = strict(new TreePath(bp, s), lc);
                for (Object o : out) {
                    checkTyped(o);
                }
                body.addAll(out);
            }
        }
        List<String> captures = new ArrayList<>();
        new TreePathScanner<Void, Void>() {
            @Override
            public Void visitVariable(VariableTree v, Void x) {
                own.add(trees.getElement(getCurrentPath()));
                return super.visitVariable(v, x);
            }

            @Override
            public Void visitIdentifier(IdentifierTree id, Void x) {
                Element e = trees.getElement(getCurrentPath());
                String n = id.getName().toString();
                boolean local = e != null && (e.getKind() == ElementKind.LOCAL_VARIABLE || e.getKind() == ElementKind.PARAMETER)
                    && !own.contains(e);
                boolean self = n.equals("this") || e != null && (e.getKind() == ElementKind.FIELD || e.getKind() == ElementKind.METHOD)
                    && !e.getModifiers().contains(Modifier.STATIC) && !(getCurrentPath().getParentPath().getLeaf() instanceof MemberSelectTree
                        && ((MemberSelectTree) getCurrentPath().getParentPath().getLeaf()).getExpression() != id);
                String c = local ? n : self ? "this" : null;
                if (c != null && !captures.contains(c)) {
                    captures.add(c);
                }
                return super.visitIdentifier(id, x);
            }
        }.scan(bp, null);
        Map<String, Object> l = kind("lambda");
        l.put("params", params);
        l.put("results", new ArrayList<Object>(frs));
        l.put("captures", new ArrayList<Object>(captures));
        l.put("body", body);
        l.put("t", ft);
        return l;
    }

    /** The statements of an expression lambda's body that gives no value: the expression as a statement. */
    private List<Object> stmtOf(TreePath ep, Ctx ctx) {
        List<Object> out = exprStmt(ep, ctx, pos(ctx.cu, ep.getLeaf()));
        for (Object o : out) {
            checkTyped(o);
        }
        return out;
    }

    /** Whether a receiver is {@code System.out}. */
    private boolean isSystemOut(TreePath recv) {
        if (recv == null || !(recv.getLeaf() instanceof MemberSelectTree)) {
            return false;
        }
        Element e = trees.getElement(recv);
        return e != null && e.getKind() == ElementKind.FIELD && e.getSimpleName().contentEquals("out")
            && ((TypeElement) e.getEnclosingElement()).getQualifiedName().contentEquals("java.lang.System");
    }

    /**
     * {@code System.out.println(x)}: {@code println}, for an integer, a boolean or a String, which
     * Java prints as cir/1's {@code println} does, then a newline (README.md, Assumptions: the line
     * separator is {@code \n}).
     */
    private Map<String, Object> println(TreePath p, MethodInvocationTree mi, Ctx ctx) {
        List<Object> args = new ArrayList<>();
        for (ExpressionTree a : mi.getArguments()) {
            TreePath ap = new TreePath(p, a);
            refusePrintedNullable(ap, ctx);
            refuseUnassigned(ap, "printed by println", "Java prints null" + STOPS);
            Map<String, Object> x = expr(ap, ctx);
            Map<String, Object> t = type(x);
            if (!isInt(t) && !t.equals(tBool()) && !t.equals(tString())) {
                throw new Unsupported("println of a " + describe(t) + ": only an integer, a boolean or a String prints alike");
            }
            args.add(x);
        }
        return intrinsic("println", args, tUnit());
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
        if ((isFloat(type(x)) || isFloat(type(y))) && constant(x) && constant(y)) {
            throw new Unsupported(op + " of two floating-point constants: Go evaluates a constant expression exactly and"
                + " rounds once, Java rounds each operation");
        }
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
                        if (!againstNull && (ot.getKind() == TypeKind.DECLARED || ot.getKind() == TypeKind.ARRAY
                            || ot.getKind() == TypeKind.TYPEVAR)
                            && (types.asElement(ot) == null || types.asElement(ot).getKind() != ElementKind.ENUM)) {
                            throw new Unsupported("== on two objects compares their identity, which cir/1 does not express");
                        }
                        if (againstNull && (ot.getKind() == TypeKind.ARRAY || qname(ot).equals("java.lang.Object"))) {
                            throw new Unsupported("an array or Object against null: the slice of no arguments is nil where Java's"
                                + " array is not null, and an any holding a nil is not nil");
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
                if (isFloat(t)) {
                    if (b.getKind() == Tree.Kind.REMAINDER) {
                        throw new Unsupported("% on a floating-point value: cir/1 % is an integer remainder");
                    }
                    if (b.getKind() != Tree.Kind.PLUS && b.getKind() != Tree.Kind.MINUS && b.getKind() != Tree.Kind.MULTIPLY
                        && b.getKind() != Tree.Kind.DIVIDE) {
                        throw new Unsupported(op + " on a " + describe(t));
                    }
                } else if (!isInt(t)) {
                    throw new Unsupported(op + " on a " + describe(t));
                }
                return binop(op, coerce(x, t), coerce(y, t), t);
        }
    }

    /**
     * {@code a << b} or {@code a >> b} of an int or a long: Java shifts by the count's low five or
     * six bits, which the read writes out as {@code b & 31} or {@code b & 63} (a literal count
     * masked as it is read), so the cir/1 shift never sees a count outside the operand's width.
     */
    private Map<String, Object> shift(TreePath p, Ctx ctx) {
        BinaryTree b = (BinaryTree) p.getLeaf();
        TreePath lp = new TreePath(p, b.getLeftOperand());
        TreePath rp = new TreePath(p, b.getRightOperand());
        String op = b.getKind() == Tree.Kind.LEFT_SHIFT ? "<<" : ">>";
        for (TreePath o : List.of(lp, rp)) {
            refuseUnassigned(o, "an operand of " + op, "Java unboxes the null" + STOPS);
        }
        Map<String, Object> t = typeOf(trees.getTypeMirror(p));
        if (!isInt(t)) {
            throw new Unsupported(op + " on a " + describe(t));
        }
        Map<String, Object> x = coerce(expr(lp, ctx), t);
        Map<String, Object> count = shiftCount(expr(rp, ctx), t);
        if ("lit".equals(x.get("k")) && !"lit".equals(count.get("k"))) {
            throw new Unsupported("a shift of a literal by a count that is not constant: Go gives the literal the type its context"
                + " wants, Java an int or a long");
        }
        return binop(op, x, count, t);
    }

    /** A shift count as Java takes it for a shift of {@code t}: its low five bits for an int, six for a long. */
    private static Map<String, Object> shiftCount(Map<String, Object> y, Map<String, Object> t) {
        Map<String, Object> yt = type(y);
        if (!isInt(yt)) {
            throw new Unsupported("a shift count of a " + describe(yt));
        }
        String width = (String) t.get("width");
        if (!width.equals("32") && !width.equals("64")) {
            throw new Unsupported("a shift of an int" + width + ", which Java shifts as an int");
        }
        long mask = width.equals("64") ? 63L : 31L;
        if ("lit".equals(y.get("k"))) {
            return lit(String.valueOf(Long.parseLong((String) y.get("value")) & mask), yt);
        }
        return binop("&", y, lit(String.valueOf(mask), yt), yt);
    }

    /**
     * Whether a read expression is a constant as Go writes it: a literal, a constant's name, or an
     * operator or conversion of constants, which Go evaluates exactly at compile time.
     */
    private static boolean constant(Map<String, Object> e) {
        Object k = e.get("k");
        if ("lit".equals(k) || "float".equals(k) || "const".equals(k)) {
            return true;
        }
        if ("binop".equals(k)) {
            return constant(sub(e, "x")) && constant(sub(e, "y"));
        }
        if ("unop".equals(k) || "conv".equals(k)) {
            return constant(sub(e, "x"));
        }
        return false;
    }

    /**
     * An integer constant expression whose value wraps (or whose shift count is masked), as the
     * literal of the value Java gives it: written as an operator of literals, Go would evaluate it
     * exactly, with no int or long to wrap in. One that does not wrap is read as it is written.
     */
    private Map<String, Object> wrappedConstant(TreePath p) {
        switch (p.getLeaf().getKind()) {
            case PARENTHESIZED:
            case INT_LITERAL:
            case LONG_LITERAL:
            case IDENTIFIER:
            case MEMBER_SELECT:
                return null;
            default:
                break;
        }
        BigInteger[] v = intConstant(p);
        if (v == null || v[0].equals(v[1])) {
            return null;
        }
        TypeKind k = trees.getTypeMirror(p).getKind();
        return lit(v[1].toString(), tInt(k == TypeKind.LONG ? "64" : "32"));
    }

    /**
     * An int or long constant expression's value: exact, and as Java computes it in its type; or
     * {@code null} when the expression is not one this evaluates.
     */
    private BigInteger[] intConstant(TreePath p) {
        Tree t = p.getLeaf();
        TypeMirror tm = trees.getTypeMirror(p);
        if (tm == null || tm.getKind() != TypeKind.INT && tm.getKind() != TypeKind.LONG) {
            return null;
        }
        boolean wide = tm.getKind() == TypeKind.LONG;
        switch (t.getKind()) {
            case PARENTHESIZED:
                return intConstant(new TreePath(p, ((ParenthesizedTree) t).getExpression()));
            case INT_LITERAL:
            case LONG_LITERAL: {
                BigInteger b = BigInteger.valueOf(((Number) ((LiteralTree) t).getValue()).longValue());
                return new BigInteger[] {b, b};
            }
            case IDENTIFIER:
            case MEMBER_SELECT: {
                Element el = trees.getElement(p);
                if (!(el instanceof VariableElement) || !(((VariableElement) el).getConstantValue() instanceof Number)) {
                    return null;
                }
                BigInteger b = BigInteger.valueOf(((Number) ((VariableElement) el).getConstantValue()).longValue());
                return new BigInteger[] {b, b};
            }
            case UNARY_MINUS:
            case UNARY_PLUS:
            case BITWISE_COMPLEMENT: {
                BigInteger[] x = intConstant(new TreePath(p, ((UnaryTree) t).getExpression()));
                if (x == null) {
                    return null;
                }
                switch (t.getKind()) {
                    case UNARY_MINUS:
                        return new BigInteger[] {x[0].negate(), wrap(x[1].negate(), wide)};
                    case UNARY_PLUS:
                        return x;
                    default:
                        return new BigInteger[] {x[0].not(), wrap(x[1].not(), wide)};
                }
            }
            case PLUS:
            case MINUS:
            case MULTIPLY:
            case DIVIDE:
            case REMAINDER:
            case AND:
            case OR:
            case XOR:
            case LEFT_SHIFT:
            case RIGHT_SHIFT: {
                BinaryTree b = (BinaryTree) t;
                BigInteger[] x = intConstant(new TreePath(p, b.getLeftOperand()));
                BigInteger[] y = intConstant(new TreePath(p, b.getRightOperand()));
                if (x == null || y == null) {
                    return null;
                }
                switch (t.getKind()) {
                    case PLUS:
                        return new BigInteger[] {x[0].add(y[0]), wrap(x[1].add(y[1]), wide)};
                    case MINUS:
                        return new BigInteger[] {x[0].subtract(y[0]), wrap(x[1].subtract(y[1]), wide)};
                    case MULTIPLY:
                        return new BigInteger[] {x[0].multiply(y[0]), wrap(x[1].multiply(y[1]), wide)};
                    case DIVIDE:
                    case REMAINDER: {
                        if (y[0].signum() == 0 || y[1].signum() == 0) {
                            return null;
                        }
                        boolean div = t.getKind() == Tree.Kind.DIVIDE;
                        return new BigInteger[] {div ? x[0].divide(y[0]) : x[0].remainder(y[0]),
                            wrap(div ? x[1].divide(y[1]) : x[1].remainder(y[1]), wide)};
                    }
                    case AND:
                        return new BigInteger[] {x[0].and(y[0]), wrap(x[1].and(y[1]), wide)};
                    case OR:
                        return new BigInteger[] {x[0].or(y[0]), wrap(x[1].or(y[1]), wide)};
                    case XOR:
                        return new BigInteger[] {x[0].xor(y[0]), wrap(x[1].xor(y[1]), wide)};
                    default: {
                        if (project) {
                            return null;
                        }
                        int count = y[1].intValue() & (wide ? 63 : 31);
                        boolean left = t.getKind() == Tree.Kind.LEFT_SHIFT;
                        BigInteger java = wrap(left ? x[1].shiftLeft(count) : x[1].shiftRight(count), wide);
                        if (y[0].signum() < 0 || y[0].bitLength() > 31) {
                            // A count Java masks has no exact shift: the read takes Java's value.
                            return new BigInteger[] {java.add(BigInteger.ONE.shiftLeft(64)), java};
                        }
                        int exactCount = y[0].intValue();
                        return new BigInteger[] {left ? x[0].shiftLeft(exactCount) : x[0].shiftRight(exactCount), java};
                    }
                }
            }
            default:
                return null;
        }
    }

    /** {@code v} wrapped to a long or an int, as Java's arithmetic leaves it. */
    private static BigInteger wrap(BigInteger v, boolean wide) {
        return BigInteger.valueOf(wide ? v.longValue() : v.intValue());
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

    /**
     * The functional interfaces of {@code java.util.function} read as cir/1 function types: the
     * interface, its one abstract method, and that method's parameter and result types as the
     * JVM type table maps them ({@code ""} for none).
     */
    private static final String[][] FUNCTIONS = {
        {"java.util.function.LongUnaryOperator", "applyAsLong", "long", "long"},
        {"java.util.function.LongBinaryOperator", "applyAsLong", "long,long", "long"},
        {"java.util.function.LongPredicate", "test", "long", "boolean"},
        {"java.util.function.LongSupplier", "getAsLong", "", "long"},
        {"java.util.function.IntUnaryOperator", "applyAsInt", "int", "int"},
        {"java.util.function.IntBinaryOperator", "applyAsInt", "int,int", "int"},
        {"java.util.function.IntPredicate", "test", "int", "boolean"},
        {"java.util.function.IntSupplier", "getAsInt", "", "int"},
        {"java.util.function.BooleanSupplier", "getAsBoolean", "", "boolean"},
    };

    private static String[] function(String qualifiedName) {
        for (String[] f : FUNCTIONS) {
            if (f[0].equals(qualifiedName)) {
                return f;
            }
        }
        return null;
    }

    private static Map<String, Object> primitiveType(String name) {
        switch (name) {
            case "long":
                return tInt("64");
            case "int":
                return tInt("32");
            default:
                return tBool();
        }
    }

    /** The cir/1 function type of a functional interface {@link #FUNCTIONS} lists. */
    private static Map<String, Object> funcType(String[] f) {
        List<Object> params = new ArrayList<>();
        if (!f[2].isEmpty()) {
            for (String p : f[2].split(",")) {
                params.add(primitiveType(p));
            }
        }
        List<Object> results = new ArrayList<>();
        if (!f[3].isEmpty()) {
            results.add(primitiveType(f[3]));
        }
        Map<String, Object> t = kind("func");
        t.put("params", params);
        t.put("results", results);
        return t;
    }

    /**
     * The cir/1 type of {@code t}: the JVM type table of {@link JvmTypes}, with the package's own
     * classes, and what only this reader maps: {@code double} and {@code float}, an array as a
     * slice, {@code Object} as {@code any}, a type variable of a generic static method as its
     * {@code param}, and the functional interfaces of {@link #FUNCTIONS} as function types.
     */
    private Map<String, Object> typeOf(TypeMirror t) {
        if (t.getKind() == TypeKind.NULL) {
            return NULL;
        }
        if (!project) {
            Map<String, Object> own = packageOnlyType(t);
            if (own != null) {
                return own;
            }
        }
        try {
            return JvmTypes.map(jvmType(t), this::ownType);
        } catch (JvmTypes.Outside o) {
            if (project && t.getKind() == TypeKind.DECLARED) {
                return externType((TypeElement) ((DeclaredType) t).asElement(), List.of());
            }
            throw new Unsupported(o.getMessage());
        }
    }

    /**
     * The types only a package read maps, or null for every other type; a project read keeps the
     * table alone, so its modules and refusals are those it has always given.
     */
    private Map<String, Object> packageOnlyType(TypeMirror t) {
        if (t.getKind() == TypeKind.DOUBLE) {
            return tFloat("64");
        }
        if (t.getKind() == TypeKind.FLOAT) {
            return tFloat("32");
        }
        if (t.getKind() == TypeKind.ARRAY) {
            TypeMirror c = ((javax.lang.model.type.ArrayType) t).getComponentType();
            return tSlice(typeOf(c));
        }
        if (t.getKind() == TypeKind.TYPEVAR) {
            Element owner = ((javax.lang.model.type.TypeVariable) t).asElement().getEnclosingElement();
            if (owner instanceof ExecutableElement && genericMethods.contains(owner)) {
                Map<String, Object> pt = kind("param");
                pt.put("name", ((javax.lang.model.type.TypeVariable) t).asElement().getSimpleName().toString());
                return pt;
            }
            throw new Unsupported(JvmTypes.outside(t.toString()));
        }
        if (t.getKind() == TypeKind.DECLARED) {
            String qn = qname(t);
            if (qn.equals("java.lang.Object")) {
                return kind("any");
            }
            String[] f = function(qn);
            if (f != null) {
                return funcType(f);
            }
        }
        return null;
    }

    // ---- projects: classes of jars and of other packages ----

    /** The extern type of the class {@code t} names, with its type arguments where the table maps them. */
    private Map<String, Object> externOf(JvmTypes.Type t) {
        TypeElement te = elements.getTypeElement(t.className());
        if (te == null) {
            return null;
        }
        List<Object> targs = new ArrayList<>();
        try {
            for (JvmTypes.Type a : t.args()) {
                targs.add(JvmTypes.map(a, this::ownType));
            }
        } catch (JvmTypes.Outside o) {
            targs.clear();
        }
        return externType(te, targs);
    }

    private static TypeElement outermost(TypeElement te) {
        Element e = te;
        while (e.getEnclosingElement() instanceof TypeElement) {
            e = e.getEnclosingElement();
        }
        return (TypeElement) e;
    }

    /** Whether the project's own sources declare {@code te}, in any package. */
    private boolean isProjectClass(TypeElement te) {
        return analysis.projectClasses.contains(outermost(te).getQualifiedName().toString());
    }

    private String packageOf(TypeElement te) {
        Element e = te;
        while (!(e instanceof PackageElement)) {
            e = e.getEnclosingElement();
        }
        return ((PackageElement) e).getQualifiedName().toString();
    }

    /**
     * Whether {@code te} is a class of another package of the project, which that package's module
     * declares: calls of it name the package, as they would a module of the set.
     */
    private boolean inOtherPackage(TypeElement te) {
        return isProjectClass(te) && annotation(te, REST_CLIENT) == null && !packageOf(te).equals(pkgName);
    }

    /**
     * The extern type of a class outside the module: for a class of another package of the project,
     * the package and the class name; for any other, the class's qualified name as its path.
     */
    private Map<String, Object> externType(TypeElement te, List<Object> targs) {
        Map<String, Object> x = Json.obj();
        x.put("k", "extern");
        x.put("path", externPath(te));
        x.put("name", externName(te));
        if (!targs.isEmpty()) {
            x.put("targs", new ArrayList<Object>(targs));
        }
        return x;
    }

    private String externPath(TypeElement te) {
        return inOtherPackage(te) ? packageOf(te) : te.getQualifiedName().toString();
    }

    /**
     * The name of an extern type: the simple name, which is also the one another package's module
     * declares a static nested class by; a class nested otherwise in another package, which its
     * module does not declare, keeps the names of the classes it is nested in ({@code Outer.Inner}).
     */
    private String externName(TypeElement te) {
        if (inOtherPackage(te) && te.getNestingKind() != javax.lang.model.element.NestingKind.TOP_LEVEL && !declared(te)) {
            String q = te.getQualifiedName().toString();
            return q.substring(packageOf(te).length() + 1);
        }
        return te.getSimpleName().toString();
    }

    private boolean externalSuperclass(TypeElement te) {
        if (!project) {
            return false;
        }
        TypeMirror sup = te.getSuperclass();
        if (sup.getKind() != TypeKind.DECLARED) {
            return false;
        }
        TypeElement se = (TypeElement) types.asElement(sup);
        TypeElement throwable = elements.getTypeElement("java.lang.Throwable");
        return !isProjectClass(se) && !types.isAssignable(types.erasure(sup), throwable.asType());
    }

    private boolean externalInterface(TypeElement ie) {
        return project && !isProjectClass(ie);
    }

    /** The first annotation of {@code e} of the class {@code qn}, or null. */
    private static AnnotationMirror annotation(Element e, String qn) {
        for (AnnotationMirror am : e.getAnnotationMirrors()) {
            if (((TypeElement) am.getAnnotationType().asElement()).getQualifiedName().contentEquals(qn)) {
                return am;
            }
        }
        return null;
    }

    /** The String value of the attribute {@code attr} of {@code am}, or null when absent or not a String. */
    private static String attribute(AnnotationMirror am, String attr) {
        for (Map.Entry<? extends ExecutableElement, ? extends javax.lang.model.element.AnnotationValue> en
            : am.getElementValues().entrySet()) {
            if (en.getKey().getSimpleName().contentEquals(attr)) {
                Object v = en.getValue().getValue();
                return v instanceof String ? (String) v : null;
            }
        }
        return null;
    }

    /** The simple name of the scope annotation of {@code te} ({@code ApplicationScoped}, …), or null. */
    private String scope(TypeElement te) {
        for (String s : SCOPES) {
            if (annotation(te, s) != null) {
                return s.substring(s.lastIndexOf('.') + 1);
            }
        }
        return null;
    }

    /** Drop every extern no call of a function or a variable's initializer the module reads names. */
    private void pruneExterns() {
        Set<String> used = new HashSet<>();
        collectExterns(outFuncs, used);
        collectExterns(outVars, used);
        externs.keySet().removeIf(k -> !used.contains(k));
    }

    private void collectExterns(Object o, Set<String> used) {
        if (o instanceof Map) {
            Map<?, ?> m = (Map<?, ?>) o;
            Object k = m.get("k");
            if ("extern".equals(k) && m.containsKey("args")) {
                used.add(externKey((String) m.get("path"), "", (String) m.get("name")));
            } else if ("extern_method".equals(k)) {
                used.add(externKey((String) m.get("path"), (String) m.get("type"), (String) m.get("name")));
            }
            for (Object v : m.values()) {
                collectExterns(v, used);
            }
        } else if (o instanceof List) {
            for (Object v : (List<?>) o) {
                collectExterns(v, used);
            }
        }
    }

    private static String externKey(String path, String type, String name) {
        return path + "\u0000" + type + "\u0000" + name;
    }

    /**
     * The declared type of an extern's parameter or result: a type that names a type variable is
     * its erasure; a class the table does not map is an extern of that class, without its type
     * arguments; an array a slice.
     */
    private Map<String, Object> signatureType(TypeMirror t) {
        TypeMirror u = mentionsVariable(t) ? types.erasure(t) : t;
        if (u.getKind() == TypeKind.ARRAY) {
            Map<String, Object> s = kind("slice");
            s.put("elem", signatureType(((javax.lang.model.type.ArrayType) u).getComponentType()));
            return s;
        }
        try {
            return typeOf(u);
        } catch (Unsupported e) {
            if (u.getKind() == TypeKind.DECLARED) {
                return externType((TypeElement) ((DeclaredType) u).asElement(), List.of());
            }
            throw e;
        }
    }

    private static boolean mentionsVariable(TypeMirror t) {
        switch (t.getKind()) {
            case TYPEVAR:
            case WILDCARD:
            case INTERSECTION:
                return true;
            case ARRAY:
                return mentionsVariable(((javax.lang.model.type.ArrayType) t).getComponentType());
            case DECLARED:
                for (TypeMirror a : ((DeclaredType) t).getTypeArguments()) {
                    if (mentionsVariable(a)) {
                        return true;
                    }
                }
                return false;
            default:
                return false;
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
        own.addAll(impls.keySet());
        own.addAll(errorLeaves.keySet());
        own.addAll(formattedLeaves.keySet());
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
            return project ? externOf(t) : null;
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
        if (impls.containsKey(te)) {
            return tNamed(te.getSimpleName().toString());
        }
        if (te.equals(errorRoot) || errorLeaves.containsKey(te) || formattedLeaves.containsKey(te)) {
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
            if (!k.equals("ptr") && !k.equals("error") && !k.equals("func") && !k.equals("any") && iface(want) == null) {
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
        // A constant converted as Java converts it: an integer constant to the nearest double or
        // float, a float constant to the double holding it exactly.
        if ("lit".equals(e.get("k")) && isInt(t) && isFloat(want)) {
            long v = Long.parseLong((String) e.get("value"));
            return want.get("bits").equals("32") ? floatLit((float) v, "32") : floatLit((double) v, "64");
        }
        if ("float".equals(e.get("k")) && t.equals(tFloat("32")) && want.equals(tFloat("64"))) {
            return floatLit(Float.parseFloat((String) e.get("value")), "64");
        }
        // Boxing a number, a boolean or a String into an Object: a cir/1 any holds the value.
        if ("any".equals(want.get("k")) && (isInt(t) || t.equals(tBool()) || t.equals(tString()))) {
            return e;
        }
        TypeElement to = iface(want);
        if (to != null) {
            return toIface(e, to, want);
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

    private static Map<String, Object> tFloat(String bits) {
        Map<String, Object> t = kind("float");
        t.put("bits", bits);
        return t;
    }

    private static boolean isFloat(Map<String, Object> t) {
        return "float".equals(t.get("k"));
    }

    /** A float literal of {@code v} at {@code bits} bits, spelled as cir/1 spells it ({@link #floatSpelling}). */
    private static Map<String, Object> floatLit(double v, String bits) {
        if (Double.isNaN(v) || Double.isInfinite(v)) {
            throw new Unsupported("a floating-point value that is not finite");
        }
        Map<String, Object> e = kind("float");
        e.put("value", floatSpelling(v, bits.equals("32")));
        e.put("t", tFloat(bits));
        return e;
    }

    /**
     * The spelling cir/1 gives a float literal ({@code float_spelling} of crates/model): the shortest
     * decimal that reads back to {@code v} at its width, the nearest of them where several are as
     * short, laid out plain when its decimal exponent {@code e} is {@code -7 < e < 21} ({@code 0.25},
     * {@code 1000}, {@code -0}), else as {@code <d>[.<digits>]e<e>} ({@code 1.5e300}).
     */
    static String floatSpelling(double v, boolean f32) {
        if (v == 0) {
            return 1 / v < 0 ? "-0" : "0";
        }
        java.math.BigDecimal exact = new java.math.BigDecimal(v);
        java.math.BigDecimal best = null;
        for (int prec = 1; prec <= 17 && best == null; prec++) {
            for (java.math.RoundingMode m : List.of(java.math.RoundingMode.HALF_EVEN, java.math.RoundingMode.FLOOR,
                java.math.RoundingMode.CEILING)) {
                java.math.BigDecimal r = exact.round(new java.math.MathContext(prec, m));
                boolean back = f32 ? Float.parseFloat(r.toString()) == (float) v : Double.parseDouble(r.toString()) == v;
                if (back && (best == null || r.subtract(exact).abs().compareTo(best.subtract(exact).abs()) < 0)) {
                    best = r;
                }
            }
        }
        String sign = best.signum() < 0 ? "-" : "";
        java.math.BigDecimal abs = best.abs().stripTrailingZeros();
        String digits = abs.unscaledValue().toString();
        int exp = digits.length() - 1 - abs.scale();
        if (-7 < exp && exp < 21) {
            int point = exp + 1;
            if (point <= 0) {
                return sign + "0." + "0".repeat(-point) + digits;
            }
            if (digits.length() <= point) {
                return sign + digits + "0".repeat(point - digits.length());
            }
            return sign + digits.substring(0, point) + "." + digits.substring(point);
        }
        String mantissa = digits.length() == 1 ? digits : digits.charAt(0) + "." + digits.substring(1);
        return sign + mantissa + "e" + exp;
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

    /**
     * List a refusal. In a project read a construct read more than once, such as a field initializer
     * each constructor starts from, is listed once: one construct, one refusal.
     */
    private void refuse(String pos, String what, String reason) {
        Map<String, Object> r = Json.obj();
        r.put("pos", pos);
        r.put("what", what);
        r.put("reason", reason);
        if (project && refusals.contains(r)) {
            return;
        }
        refusals.add(r);
    }

    private void refuseDecl(CompilationUnitTree cu, Tree t, ModifiersTree mods, String what, String reason) {
        refuse(declPos(cu, t, mods), what, reason);
    }

    private void refuseAnnotations(CompilationUnitTree cu, ModifiersTree mods) {
        refuseAnnotations(cu, mods, false);
    }

    /** With {@code persistence}, a class or instance field of a project read, whose JPA annotations {@link #persistence} records. */
    private void refuseAnnotations(CompilationUnitTree cu, ModifiersTree mods, boolean persistence) {
        refuseAnnotations(cu, mods, persistence, Set.of());
    }

    /** With {@code mapped}, the annotations the declaration maps besides the Quarkus ones ({@code @Provider} on an exception mapper). */
    private void refuseAnnotations(CompilationUnitTree cu, ModifiersTree mods, boolean persistence, Set<String> mapped) {
        for (AnnotationTree a : mods.getAnnotations()) {
            TypeMirror at = trees.getTypeMirror(new TreePath(new TreePath(cu), a.getAnnotationType()));
            String qn = at == null ? "" : qname(at);
            if (project && persistence && PERSISTENCE.contains(qn) || mapped.contains(qn)) {
                continue;
            }
            if (!HARMLESS_ANNOTATIONS.contains(qn) && !(project && QUARKUS_ANNOTATIONS.contains(qn))) {
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

package codespec.read.groovy;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.OutputStream;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.FileObject;
import javax.tools.ForwardingJavaFileManager;
import javax.tools.JavaCompiler;
import javax.tools.JavaFileManager;
import javax.tools.JavaFileObject;
import javax.tools.SimpleJavaFileObject;
import javax.tools.StandardJavaFileManager;
import javax.tools.ToolProvider;

/**
 * The {@code src/java} files of a Grails application, compiled with javac in memory so that the
 * Groovy files that name their classes resolve. They are resolve-only: never read. A file javac
 * reports an error in is left out and the rest compiled again, as a resolve-only Groovy file whose
 * imports do not resolve is left out; a Groovy file that names one of its classes is then refused
 * {@code unresolved}. The Java files see the Groovy jar and the jars of {@code
 * CODESPEC_GROOVY_CLASSPATH}, not the Groovy files; {@code parent} must see those jars too, since a
 * class that extends one of theirs is defined under it.
 */
final class JavaSources {
    private JavaSources() {}

    /** A class loader under {@code parent} that defines the classes {@code files} compile to. */
    static ClassLoader compile(List<Path> files, List<Path> classpath, ClassLoader parent) throws Reader.Failure {
        if (files.isEmpty()) {
            return parent;
        }
        JavaCompiler javac = ToolProvider.getSystemJavaCompiler();
        if (javac == null) {
            throw new Reader.Failure("src/java: no Java compiler in this runtime; run the reader on a JDK");
        }
        List<String> cp = new ArrayList<>();
        for (Path p : classpath) {
            cp.add(p.toAbsolutePath().normalize().toString());
        }
        try {
            // The Groovy jar, not the reader's own classes.
            cp.add(Path.of(groovy.lang.GroovyObject.class.getProtectionDomain().getCodeSource().getLocation().toURI()).toString());
        } catch (java.net.URISyntaxException | RuntimeException e) {
            throw new Reader.Failure("src/java: the Groovy jar's location: " + e);
        }
        List<Path> remaining = new ArrayList<>(files);
        while (!remaining.isEmpty()) {
            DiagnosticCollector<JavaFileObject> diags = new DiagnosticCollector<>();
            StandardJavaFileManager std = javac.getStandardFileManager(diags, null, StandardCharsets.UTF_8);
            Memory mem = new Memory(std);
            List<File> sources = new ArrayList<>();
            for (Path p : remaining) {
                sources.add(p.toFile());
            }
            List<String> options = List.of("-proc:none", "-nowarn", "-classpath", String.join(File.pathSeparator, cp));
            boolean ok = javac.getTask(null, mem, diags, options, null, std.getJavaFileObjectsFromFiles(sources)).call();
            if (ok) {
                return new Defined(parent, mem.classes);
            }
            Set<Path> bad = new HashSet<>();
            for (Diagnostic<? extends JavaFileObject> d : diags.getDiagnostics()) {
                if (d.getKind() == Diagnostic.Kind.ERROR && d.getSource() != null) {
                    bad.add(Path.of(d.getSource().toUri()).toAbsolutePath().normalize());
                }
            }
            if (bad.isEmpty() || !remaining.removeAll(bad)) {
                throw new Reader.Failure("src/java: javac failed: " + diags.getDiagnostics());
            }
        }
        return parent;
    }

    /** Class files written to memory, by binary name. */
    private static final class Memory extends ForwardingJavaFileManager<JavaFileManager> {
        final Map<String, ByteArrayOutputStream> classes = new HashMap<>();

        Memory(JavaFileManager m) {
            super(m);
        }

        @Override
        public JavaFileObject getJavaFileForOutput(Location location, String className, JavaFileObject.Kind kind, FileObject sibling) {
            return new SimpleJavaFileObject(URI.create("mem:///" + className.replace('.', '/') + kind.extension), kind) {
                @Override
                public OutputStream openOutputStream() {
                    ByteArrayOutputStream out = new ByteArrayOutputStream();
                    classes.put(className, out);
                    return out;
                }
            };
        }
    }

    /** Defines the compiled classes on demand. */
    private static final class Defined extends ClassLoader {
        private final Map<String, ByteArrayOutputStream> classes;

        Defined(ClassLoader parent, Map<String, ByteArrayOutputStream> classes) {
            super(parent);
            this.classes = classes;
        }

        @Override
        protected Class<?> findClass(String name) throws ClassNotFoundException {
            ByteArrayOutputStream b = classes.get(name);
            if (b == null) {
                throw new ClassNotFoundException(name);
            }
            byte[] bytes = b.toByteArray();
            return defineClass(name, bytes, 0, bytes.length);
        }
    }
}

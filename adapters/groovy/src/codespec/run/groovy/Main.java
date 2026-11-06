package codespec.run.groovy;

import codespec.run.Runner;
import java.nio.file.Path;
import java.util.List;
import org.codehaus.groovy.control.CompilationFailedException;
import org.codehaus.groovy.control.CompilationUnit;
import org.codehaus.groovy.control.CompilerConfiguration;

/**
 * {@code codespec-run-groovy [--build <dir>] <package dir> <calls.json>}: compiles the {@code
 * .groovy} files of one package directory with the Groovy 2.4 compiler on the class path, to class
 * files, and makes the calls against them, as {@link Runner} says.
 */
public final class Main {
    private Main() {}

    /** Groovy's own compiler, every phase through class generation. */
    static final class Groovyc implements Runner.Compiler {
        @Override
        public String extension() {
            return ".groovy";
        }

        @Override
        public void compile(List<Path> sources, Path classes) throws Runner.Failed {
            CompilerConfiguration conf = new CompilerConfiguration();
            conf.setTargetDirectory(classes.toFile());
            CompilationUnit unit = new CompilationUnit(conf);
            for (Path p : sources) {
                unit.addSource(p.toFile());
            }
            try {
                unit.compile();
            } catch (CompilationFailedException e) {
                throw new Runner.Failed("groovyc: " + e.getMessage());
            }
        }
    }

    public static void main(String[] args) {
        Runner.main("codespec-run-groovy", new Groovyc(), args);
    }
}

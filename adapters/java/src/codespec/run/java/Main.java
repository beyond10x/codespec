package codespec.run.java;

import codespec.run.Runner;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import javax.tools.JavaCompiler;
import javax.tools.ToolProvider;

/**
 * {@code codespec-run-java [--build <dir>] <package dir> <calls.json>}: compiles the {@code .java}
 * files of one package directory with the JDK's compiler ({@code --release 17}) and makes the calls
 * against them, as {@link Runner} says.
 */
public final class Main {
    private Main() {}

    /** The JDK's compiler, through {@code javax.tools}. */
    static final class Javac implements Runner.Compiler {
        @Override
        public String extension() {
            return ".java";
        }

        @Override
        public void compile(List<Path> sources, Path classes) throws Runner.Failed {
            JavaCompiler javac = ToolProvider.getSystemJavaCompiler();
            if (javac == null) {
                throw new Runner.Failed("no Java compiler: run on a JDK");
            }
            List<String> args = new ArrayList<>(List.of("--release", "17", "-proc:none", "-nowarn", "-d", classes.toString()));
            for (Path p : sources) {
                args.add(p.toString());
            }
            int code = javac.run(null, System.err, System.err, args.toArray(new String[0]));
            if (code != 0) {
                throw new Runner.Failed("javac exited " + code);
            }
        }
    }

    public static void main(String[] args) {
        Runner.main("codespec-run-java", new Javac(), args);
    }
}

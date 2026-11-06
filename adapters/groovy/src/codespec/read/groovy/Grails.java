package codespec.read.groovy;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Properties;
import java.util.stream.Stream;

/**
 * A Grails 2.5.6 application, found by its {@code application.properties}: the artefacts the
 * reader reads and the files it compiles only so that references to them resolve.
 *
 * <p>Read: every {@code .groovy} file under {@code grails-app/domain} (a domain class) and every
 * {@code *Service.groovy} under {@code grails-app/services} (a service). Resolve-only: every other
 * {@code .groovy} file under {@code grails-app} ({@code utils}, {@code taglib}, {@code jobs}, {@code
 * conf}, {@code controllers}) and under {@code src}. Plugins are not read.
 */
final class Grails {
    static final String VERSION = "2.5.6";

    final Path root;
    /** The module name: {@code app.name}. */
    final String name;
    final List<Path> domain = new ArrayList<>();
    final List<Path> services = new ArrayList<>();
    final List<Path> resolveOnly = new ArrayList<>();

    private Grails(Path root, String name) {
        this.root = root;
        this.name = name;
    }

    /** Whether {@code dir} is a Grails application: it holds {@code application.properties}. */
    static boolean isApplication(Path dir) {
        return Files.isRegularFile(dir.resolve("application.properties"));
    }

    /** The application in {@code dir}, which must be a Grails 2.5.6 application. */
    static Grails of(Path dir) throws Reader.Failure {
        Path root = dir.toAbsolutePath().normalize();
        Properties p = new Properties();
        try (InputStream in = Files.newInputStream(root.resolve("application.properties"))) {
            p.load(in);
        } catch (IOException e) {
            throw new Reader.Failure(root.resolve("application.properties") + ": " + e.getMessage());
        }
        String version = p.getProperty("app.grails.version");
        if (version == null) {
            throw new Reader.Failure("application.properties: no app.grails.version: not a Grails application");
        }
        if (!version.trim().equals(VERSION)) {
            throw new Reader.Failure("application.properties: app.grails.version=" + version.trim() + ": only Grails "
                + VERSION + " is read");
        }
        String name = identifier(p.getProperty("app.name", ""));
        if (name.isEmpty()) {
            throw new Reader.Failure("application.properties: app.name is not set");
        }
        Grails g = new Grails(root, name);
        for (Path f : groovyFiles(root.resolve("grails-app"))) {
            String rel = g.relative(f);
            if (rel.startsWith("grails-app/domain/")) {
                g.domain.add(f);
            } else if (rel.startsWith("grails-app/services/") && f.getFileName().toString().endsWith("Service.groovy")) {
                g.services.add(f);
            } else {
                g.resolveOnly.add(f);
            }
        }
        g.resolveOnly.addAll(groovyFiles(root.resolve("src")));
        if (g.domain.isEmpty() && g.services.isEmpty()) {
            throw new Reader.Failure(root + ": no domain class and no service under grails-app");
        }
        return g;
    }

    /** The files the reader reads: domain classes, then services, each in path order. */
    List<Path> read() {
        List<Path> out = new ArrayList<>(domain);
        out.addAll(services);
        return out;
    }

    /** A file's path relative to the application, with {@code /} separators: its name in every position. */
    String relative(Path f) {
        Path r = root.relativize(f.toAbsolutePath().normalize());
        List<String> parts = new ArrayList<>();
        for (Path part : r) {
            parts.add(part.toString());
        }
        return String.join("/", parts);
    }

    private static List<Path> groovyFiles(Path dir) throws Reader.Failure {
        if (!Files.isDirectory(dir)) {
            return List.of();
        }
        try (Stream<Path> s = Files.walk(dir)) {
            return s.filter(p -> Files.isRegularFile(p) && p.getFileName().toString().endsWith(".groovy"))
                .map(p -> p.toAbsolutePath().normalize())
                .sorted(Comparator.comparing(Path::toString))
                .toList();
        } catch (IOException e) {
            throw new Reader.Failure(dir + ": " + e.getMessage());
        }
    }

    /** {@code app.name} as a module name: every character outside {@code [A-Za-z0-9_]} becomes {@code _}. */
    private static String identifier(String s) {
        StringBuilder b = new StringBuilder();
        for (char c : s.trim().toCharArray()) {
            b.append(c < 128 && (Character.isLetterOrDigit(c) || c == '_') ? c : '_');
        }
        if (b.length() > 0 && Character.isDigit(b.charAt(0))) {
            b.insert(0, '_');
        }
        return b.toString();
    }
}

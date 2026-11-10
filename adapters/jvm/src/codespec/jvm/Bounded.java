package codespec.jvm;

/**
 * A JVM reader's {@code main} run on a thread of a stated stack, {@link #STACK_BYTES}. Neither
 * javac nor Groovy's parser bounds nesting depth: each recurses a level per parenthesis, and so do
 * the readers over the trees they build. A source nested deeper than that stack holds is a
 * refusal, exit 1 with one line naming the reader, not a crash: a {@link StackOverflowError},
 * thrown or the cause of what javac throws, ends the run there.
 */
public final class Bounded {
    private Bounded() {}

    /** The stack a reader runs on: 64 MiB, 64 times the JVM's default. */
    public static final long STACK_BYTES = 64L << 20;

    /** Run {@code main} as the reader {@code reader} (its program name) on a stack of {@link #STACK_BYTES}. */
    public static void run(String reader, Runnable main) {
        Throwable[] thrown = new Throwable[1];
        Thread t = new Thread(null, () -> {
            try {
                main.run();
            } catch (Throwable e) {
                thrown[0] = e;
            }
        }, reader, STACK_BYTES);
        t.start();
        while (true) {
            try {
                t.join();
                break;
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
        }
        Throwable e = thrown[0];
        if (e == null) {
            return;
        }
        if (overflow(e)) {
            System.err.println(reader + ": the source nests deeper than the reader's stack of "
                + (STACK_BYTES >> 20) + " MiB holds");
            System.exit(1);
        }
        if (e instanceof RuntimeException r) {
            throw r;
        }
        if (e instanceof Error r) {
            throw r;
        }
        throw new IllegalStateException(e);
    }

    /** Whether {@code e} is a stack overflow or was caused by one. */
    public static boolean overflow(Throwable e) {
        for (Throwable c = e; c != null; c = c.getCause()) {
            if (c instanceof StackOverflowError) {
                return true;
            }
            if (c.getCause() == c) {
                break;
            }
        }
        return false;
    }
}

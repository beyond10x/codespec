package decl_generic;

/**
 * The Java source of the kit case decl-generic: a generic static method over any type, its type
 * argument the one javac infers.
 */
public final class Case {
    static <T> T pick(boolean second, T a, T b) {
        if (second) {
            return b;
        }
        return a;
    }

    /** Picks n and n+1 through a method over any type and sums them. */
    public static long sizes(long n) {
        long a = pick(false, n, n + 1L);
        long b = pick(true, n, n + 1L);
        return a + b;
    }
}

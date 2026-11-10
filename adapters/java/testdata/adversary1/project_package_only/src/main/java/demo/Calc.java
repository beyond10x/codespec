package demo;

/**
 * A project whose methods use a shift, a labeled break and float literals: README.md and the
 * Reader's class comment say the kit work maps these in a package read only and a project's
 * modules are read as before, which refused each at the unit's base.
 */
public final class Calc {
    /** Shifts left. */
    public static long shl(long a, long b) {
        return a << b;
    }

    /** Counts to three through a labeled break. */
    public static long find(long n) {
        long hits = 0;
        loop:
        for (long i = 0; i < n; i++) {
            if (i == 3) {
                break loop;
            }
            hits++;
        }
        return hits;
    }

    /** Compares two float literals. */
    public static boolean less(long n) {
        return 0.1 < 0.2;
    }
}

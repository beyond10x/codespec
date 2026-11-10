package compound_order;

/**
 * A compound assignment whose right operand changes its target. Java reads the target before it
 * evaluates the right operand (JLS 15.26.2); Go's {@code x op= f()} reads {@code x} after the call.
 * The array element and the module-level variable are new in the unit; {@code fieldOrder}, the
 * same through a field, reads the same way at the unit's base.
 */
public final class Case {
    static long total;

    /** An array element: Java gives 1 + 1. */
    public static long arrayOrder(long n) {
        long[] a = {1, 2};
        a[0] += bump(a);
        return a[0];
    }

    static long bump(long[] a) {
        a[0] = 100;
        return 1;
    }

    /** A module-level variable: Java gives 1 + 1. */
    public static long globalOrder(long n) {
        total = 1;
        total += reset();
        return total;
    }

    static long reset() {
        total = 100;
        return 1;
    }

    /** A field: Java gives 1 + 1. */
    public static long fieldOrder(long n) {
        Box b = new Box();
        b.v = 1;
        b.v += b.bump();
        return b.v;
    }
}

/** Holds a value its own method overwrites. */
final class Box {
    long v;

    long bump() {
        v = 100;
        return 1;
    }
}

package global_shadow;

/**
 * A static field read through its class where a local or a parameter has its name. Java names the
 * field; the read is {@code global count} beside {@code var count}, and written as Go both are the
 * identifier {@code count}, which names the local.
 */
public final class Case {
    static long count = 10;

    /** Adds a local named as the field to the field. */
    public static long add(long n) {
        long count = n;
        Case.count += count;
        return Case.count;
    }

    /** The field from another class whose parameter has its name. */
    public static long other(long count) {
        return Holder.read(count);
    }
}

/** Reads the field of Case. */
final class Holder {
    static long read(long count) {
        return Case.count * 100 + count;
    }
}

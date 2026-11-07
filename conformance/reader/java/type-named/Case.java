package type_named;

/** The Java source of the kit case type-named. */
public final class Case {
    /** Holds n in a Meters and reads it back. */
    public static long measure(long n) {
        Meters m = new Meters(n);
        return m.value;
    }
}

/** A length. */
final class Meters {
    long value;

    Meters(long value) {
        this.value = value;
    }
}

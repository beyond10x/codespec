package sem_int_wrap

/** The Groovy source of the kit case sem-int-wrap: a long wraps as Go's int64 does, and divides with intdiv. */
final class Case {
    /** Adds, wrapping on overflow. */
    static long add(long a, long b) {
        return a + b
    }

    /** Multiplies, wrapping on overflow. */
    static long mul(long a, long b) {
        return a * b
    }

    /** Divides; Long.MIN_VALUE.intdiv(-1) wraps to Long.MIN_VALUE. */
    static long div(long a, long b) {
        return a.intdiv(b)
    }

    /** The remainder; Long.MIN_VALUE % -1 is 0. */
    static long rem(long a, long b) {
        return a % b
    }

    /** Negates; -Long.MIN_VALUE wraps to Long.MIN_VALUE. */
    static long neg(long a) {
        return -a
    }
}

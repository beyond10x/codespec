package assign_div

/** The Groovy source of the kit case assign-div: Groovy has no compound intdiv, so x = x.intdiv(y) is its x /= y. */
final class Case {
    /** Divides in place. */
    static long divBy(long a, long b) {
        a = a.intdiv(b)
        return a
    }
}

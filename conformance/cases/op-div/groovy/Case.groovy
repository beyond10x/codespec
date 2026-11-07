package op_div

/** The Groovy source of the kit case op-div: Groovy divides integers with intdiv, since its / on two integers is a BigDecimal division. */
final class Case {
    /** Divides, truncating toward zero. */
    static long div(long a, long b) {
        return a.intdiv(b)
    }
}

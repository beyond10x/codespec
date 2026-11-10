package type_float_compare

/** The Groovy source of the kit case type-float-compare: Groovy adds and divides two floats as doubles, so each float result is cast back. */
final class Case {
    /** Tells whether a tenth plus two tenths is three tenths: in doubles it is not, in floats it is. */
    static boolean exact(boolean wide) {
        if (wide) {
            double x = 0.1d
            double y = 0.2d
            return x + y == 0.3d
        }
        float a = 0.1f
        float b = 0.2f
        return (float) (a + b) == 0.3f
    }

    /** Tells whether a third of one, or of 1e21, times three is the number again, in floats. */
    static boolean third(boolean big) {
        float y = 1f
        if (big) {
            y = 1e21f
        }
        float t = (float) (y / 3f)
        return (float) (t * 3f) == y
    }
}

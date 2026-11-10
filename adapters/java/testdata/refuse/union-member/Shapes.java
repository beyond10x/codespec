package union_member;

/** A sealed interface whose record Square declares a method: a cir/1 union variant has fields only. */
public final class Shapes {
    /** The method beside them. */
    public static long one() {
        return 1L;
    }
}

/** Shape is a circle or a square. */
sealed interface Shape permits Circle, Square {} // refused

record Circle(long radius) implements Shape {} // refused

record Square(long side) implements Shape { // refused
    long area() {
        return side * side;
    }
}

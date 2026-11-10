package iface_reassign;

/**
 * A local of an interface with two implementations, given one and then the other. The read is
 * right: {@code let a = to_iface(NewBox(n)) : Shape}. The Go writer drops the conversion in the
 * {@code let} ({@code a := NewBox(n)}), so {@code a = b} does not compile.
 */
public final class Case {
    /** The area of the second shape a local held. */
    public static long last(long n) {
        Shape a = new Box(n);
        Shape b = new Disc(n);
        a = b;
        return a.area();
    }
}

interface Shape {
    long area();
}

final class Box implements Shape {
    long side;

    Box(long side) {
        this.side = side;
    }

    public long area() {
        return side * side;
    }
}

final class Disc implements Shape {
    long r;

    Disc(long r) {
        this.r = r;
    }

    public long area() {
        return 3 * r * r;
    }
}

package decl_interface;

/**
 * The Java source of the kit case decl-interface: an interface of the package with two
 * implementations is a cir/1 interface.
 */
public final class Case {
    static long area(Shape s) {
        return s.area();
    }

    /** Sums, through Shape, the area of a square of side n and of a rectangle n by 2. */
    public static long areas(long n) {
        return area(new Square(n)) + area(new Rect(n, 2L));
    }
}

/** Has an area. */
interface Shape {
    long area();
}

final class Square implements Shape {
    final long side;

    Square(long side) {
        this.side = side;
    }

    public long area() {
        return side * side;
    }
}

final class Rect implements Shape {
    final long w;
    final long h;

    Rect(long w, long h) {
        this.w = w;
        this.h = h;
    }

    public long area() {
        return w * h;
    }
}

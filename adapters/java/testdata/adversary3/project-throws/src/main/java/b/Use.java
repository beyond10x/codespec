package b;

import b.lib.Lib;
import b.lib.LibError;
import b.lib.Outer;
import b.lib.Shape;
import b.lib.Sub;
import b.lib.Two;

public class Use {
    public static long total(long k) throws LibError {
        long a = Lib.load(k);
        return a + 1;
    }

    public static boolean found(long k) {
        return Lib.find(k).isPresent();
    }

    public static long viaSub(Sub s) {
        return s.v;
    }

    public static Outer.In inner(Outer.In i) {
        return i;
    }

    public static long area(Shape s) {
        return s.area();
    }

    public static long refusedOverload() {
        return Two.twice("x");
    }
}

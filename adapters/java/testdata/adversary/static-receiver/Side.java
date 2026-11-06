package staticreceiver;

/** Java evaluates the receiver expression of a static call through an instance, then ignores its value. */
public class Side {
    private int n;

    public Side bump() {
        n++;
        return this;
    }

    public static int zero() {
        return 0;
    }

    /** Java returns 1: bump() runs before zero(). */
    @SuppressWarnings("static-access")
    public int run() {
        int z = bump().zero();
        return n + z;
    }
}

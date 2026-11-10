package b.lib;

/** b.lib refuses this interface (a default method) and declares no interface for it. */
public interface Shape {
    long area();

    default long twice() {
        return area() * 2;
    }
}

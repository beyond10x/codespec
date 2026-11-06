package shapes;

public class Disc implements Shape { // refused
    private long radius;

    public long area() {
        return 3 * radius * radius;
    }
}

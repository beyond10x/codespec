package shapes;

public class Box implements Shape {
    private long side;

    public long area() {
        return side * side;
    }
}

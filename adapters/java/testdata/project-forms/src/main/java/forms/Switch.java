package forms;

public class Switch {
    public boolean on(Lamp l) {
        return l.lit; // refused: Panache
    }

    public boolean low(Lamp l) {
        return l.dim;
    }
}

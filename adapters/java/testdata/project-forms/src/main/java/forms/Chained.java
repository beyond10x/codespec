package forms;

public class Chained {
    public String label = "start";

    public Chained() { // refused: this(
        this("none");
    }

    public Chained(String label) {
        this.label = label;
    }
}

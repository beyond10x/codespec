package shop;

public class Pair {
    public long a = 1;
    public long b = a + 1; // refused: this where there is no object

    public Pair() {
    }

    public Pair(long a) {
        this.a = a;
    }

    public Pair(String tag) {
    }
}

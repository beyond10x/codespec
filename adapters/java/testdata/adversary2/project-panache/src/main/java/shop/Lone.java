package shop;

public class Lone {
    public long n;

    public <T> Lone(T seed) { // refused: generic
        this.n = 1;
    }
}

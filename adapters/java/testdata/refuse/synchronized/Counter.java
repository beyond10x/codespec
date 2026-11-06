package counter;

public class Counter {
    private int count;

    public void bump() {
        synchronized (this) { // refused
            count++;
        }
    }

    public int count() {
        return count;
    }
}

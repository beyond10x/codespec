package forms;

public class Inner {
    long base;

    public class Part { // refused: inner class
        long size;
    }
}

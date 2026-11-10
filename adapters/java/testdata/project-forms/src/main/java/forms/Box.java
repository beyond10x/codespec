package forms;

public class Box {
    public long size;

    public Box() {
        this.size = 1;
    }

    public Box(long size) { // refused: a second function named NewBox2
        this.size = size;
    }
}

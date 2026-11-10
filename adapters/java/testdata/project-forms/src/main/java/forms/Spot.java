package forms;

import java.awt.Point;

public class Spot extends Point {
    private static final long serialVersionUID = 1L;

    public int across() {
        return x; // refused: field of java.awt.Point, which is not read
    }
}

package shop;

public class Desk {
    public long across(Spot s) {
        return s.xCoord; // refused: Panache
    }

    public void fill(Gauge g) {
        g.level = 5; // refused: Panache
    }
}

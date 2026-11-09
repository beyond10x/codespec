package shop.pricing;

public class Price {
    private final int unit;

    public Price(int unit) {
        this.unit = unit;
    }

    public int apply(int qty) {
        return unit * qty;
    }
}

package shop;

import shop.pricing.Price;

public class Cart {
    public int total(Price price, int qty) {
        return price.apply(qty);
    }
}

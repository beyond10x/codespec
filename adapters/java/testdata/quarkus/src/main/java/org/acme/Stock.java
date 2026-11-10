package org.acme;

public class Stock {
    public boolean stored(Fruit f) {
        return f.id != null;
    }

    public boolean named(Fruit f) {
        return f.name != null;
    }
}

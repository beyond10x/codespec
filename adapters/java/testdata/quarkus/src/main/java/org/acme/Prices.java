package org.acme;

public class Prices {
    public static class Quote {
        public long cents;

        public Quote(long cents) {
            this.cents = cents;
        }
    }

    public Quote quote(long cents) {
        return new Quote(cents);
    }
}

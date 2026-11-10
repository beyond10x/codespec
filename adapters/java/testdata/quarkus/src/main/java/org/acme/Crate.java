package org.acme;

public class Crate {
    public String label = "crate";
    public long size;

    public Crate() {
    }

    public Crate(String label) {
        this.label = label;
        this.size = 1;
    }

    public static Crate of(String label) {
        return new Crate(label);
    }
}

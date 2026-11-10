package forms.other;

public class Kit {
    public long n;

    public Kit(long n) {
        this.n = n;
    }

    public static Kit of(long n) {
        return new Kit(n);
    }

    public long size() {
        return n;
    }

    public Thing thing() {
        return new Thing();
    }

    public String label(String prefix) {
        return prefix;
    }
}

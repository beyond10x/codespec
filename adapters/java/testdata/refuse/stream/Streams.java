package streams;

import java.util.ArrayList;
import java.util.List;

public class Streams {
    private final List<Long> xs = new ArrayList<>();

    public long distinct() {
        long n = xs.stream().distinct().count(); // refused
        return n;
    }
}

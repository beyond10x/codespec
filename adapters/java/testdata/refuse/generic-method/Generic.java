package generic;

import java.util.List;

public class Generic {
    public static int one() {
        return 1;
    }

    public static <T> T first(List<T> xs) { // refused
        return xs.get(0);
    }
}

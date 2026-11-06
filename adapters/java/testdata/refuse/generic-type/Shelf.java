package generictype;

import java.util.List;
import java.util.Map;
import java.util.Set;

public class Shelf {
    public static int count() {
        return 1;
    }

    public static int ratings(Map<String, List<Double>> byTitle) {
        return byTitle.size();
    }

    public static int tags(Set<String> names) {
        return names.size();
    }
}

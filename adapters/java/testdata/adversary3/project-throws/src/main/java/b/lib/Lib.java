package b.lib;

import java.util.Optional;

public class Lib {
    public static long load(long k) throws LibError {
        if (k < 0) {
            throw new Negative();
        }
        return k * 2;
    }

    public static Optional<Long> find(long k) {
        return Optional.of(k);
    }
}

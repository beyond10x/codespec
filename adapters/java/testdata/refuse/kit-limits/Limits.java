package limits;

import java.util.function.LongUnaryOperator;

public class Limits {
    static String name; // refused
    static long start = Long.parseLong("1"); // refused
    static long count = 3;

    static <T> boolean same(T a, T b) {
        return a == b; // refused
    }

    static long unsigned(long x) {
        return x >>> 3; // refused
    }

    static long outer(long n) {
        long hits = 0;
        rows:
        for (long i = 0; i < n; i++) {
            for (long j = 0; j < n; j++) {
                if (j > i) {
                    continue rows; // refused
                }
                hits++;
            }
        }
        return hits;
    }

    static long block(long n) {
        done: { // refused
            if (n > 0) {
                break done;
            }
        }
        return n;
    }

    static double rest(double a, double b) {
        return a % b; // refused
    }

    static boolean missing(long[] xs) {
        return xs == null; // refused
    }

    static int sized(int n) {
        long[] xs = new long[n]; // refused
        return xs.length;
    }

    static void show(double d) {
        System.out.println(d); // refused
    }

    static int narrow(long x) {
        return 7;
    }

    static LongUnaryOperator converted() {
        return Limits::narrow; // refused
    }

    static long readable(long x) {
        return x + count;
    }
}

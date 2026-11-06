package sem_continue_post;

/** The Java source of the kit case sem-continue-post. */
public final class Case {
    /** Sums 0 up to n-1 except skip; the continue still runs i++. */
    public static long skipOne(long n, long skip) {
        long s = 0L;
        for (long i = 0L; i < n; i++) {
            if (i == skip) {
                continue;
            }
            s += i;
        }
        return s;
    }
}

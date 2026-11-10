package sem_switch_break;

/**
 * The Java source of the kit case sem-switch-break: a {@code break} naming the loop's label, inside
 * a switch, leaves the loop, as a cir/1 {@code break} inside a switch does. A Java case label is a
 * constant, so the switch is on {@code i - stop}.
 */
public final class Case {
    /** Counts up to n but stops at stop; the break leaves the loop, not only the switch. */
    public static int until(int n, int stop) {
        int i = 0;
        loop:
        while (i < n) {
            switch (i - stop) {
                case 0:
                    break loop;
                default:
                    break;
            }
            i++;
        }
        return i;
    }
}

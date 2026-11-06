package nullorder;

/**
 * An assignment to a field that holds null until assigned, through a receiver that is a call: Java
 * evaluates the receiver {@code s.pick()} before the value {@code s.stamp()}, so the label is
 * "receiver first".
 */
public class Shelf {
    private String label;
    private int picks;
    private Shelf next;

    Shelf pick() {
        picks = picks + 1;
        return next;
    }

    String stamp() {
        if (picks == 0) {
            return "value first";
        }
        return "receiver first";
    }

    public static String labelAfterPick() {
        Shelf s = new Shelf();
        s.next = new Shelf();
        s.pick().label = s.stamp();
        return s.next.label;
    }
}

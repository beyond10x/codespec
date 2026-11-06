package nullable;

/** A note whose text, count and next note hold null until written, as a Java reference field without an initializer does. */
public class Note {
    private String text;
    private Integer count;
    private Note next;

    public void write(String t) {
        this.text = t;
    }

    /** Writes t, then changes the parameter: the field keeps what was written. */
    public void writeThenChange(String t) {
        this.text = t;
        t = "changed";
    }

    public void clear() {
        this.text = null;
    }

    public void tally(int n) {
        this.count = n;
    }

    public boolean hasText() {
        return text != null;
    }

    public boolean hasCount() {
        return this.count != null;
    }

    /** Links the note after another. */
    public static void link(Note from, Note to) {
        from.next = to;
    }

    public boolean hasNext() {
        return next != null;
    }

    public String read() {
        return text;
    }

    public int counted() {
        return count;
    }

    /** The text, or the fallback while there is none. */
    public String textOr(String fallback) {
        if (text == null) {
            return fallback;
        }
        return text;
    }

    /** Gives the second note the first one's text, null or not. */
    public static void copyText(Note from, Note to) {
        to.text = from.text;
    }

    public static boolean freshHasText() {
        return new Note().hasText();
    }

    public static boolean hasTextAfterWrite(String t) {
        Note n = new Note();
        n.write(t);
        return n.hasText();
    }

    public static String readAfterWrite(String t) {
        Note n = new Note();
        n.write(t);
        return n.read();
    }

    public static String readAfterWriteThenChange(String t) {
        Note n = new Note();
        n.writeThenChange(t);
        return n.read();
    }

    public static String fallbackWhenFresh(String fallback) {
        return new Note().textOr(fallback);
    }

    public static String fallbackAfterWrite(String t, String fallback) {
        Note n = new Note();
        n.write(t);
        return n.textOr(fallback);
    }

    public static boolean hasTextAfterClear(String t) {
        Note n = new Note();
        n.write(t);
        n.clear();
        return n.hasText();
    }

    public static boolean hasTextAfterCopyOfFresh(String t) {
        Note fresh = new Note();
        Note n = new Note();
        n.write(t);
        copyText(fresh, n);
        return n.hasText();
    }

    /** A copy keeps the text it took when its source is written again. */
    public static String copyAfterSourceRewritten(String t, String u) {
        Note a = new Note();
        a.write(t);
        Note b = new Note();
        copyText(a, b);
        a.write(u);
        return b.read() + "/" + a.read();
    }

    public static boolean freshHasNext() {
        return new Note().hasNext();
    }

    /** The next note's text, read through the link. */
    public static String nextTextAfterLink(String t) {
        Note a = new Note();
        Note b = new Note();
        b.write(t);
        link(a, b);
        return a.next.read();
    }

    public static boolean freshHasCount() {
        return new Note().hasCount();
    }

    public static int countAfterTally(int n) {
        Note note = new Note();
        note.tally(n);
        return note.counted();
    }
}

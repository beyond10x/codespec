package nullfield;

/** Fields left at Java's default: null for a String, a boxed number and an enum, not "", 0 or "". */
public class Label {
    private String text;
    private Integer count;
    private Shade shade;

    public void set(String t, int n, Shade s) {
        this.text = t;
        this.count = n;
        this.shade = s;
    }

    /** On a fresh Label Java returns "text=null count=null shade=null". */
    public String show() {
        return "text=" + text + " count=" + count + " shade=" + shade;
    }
}

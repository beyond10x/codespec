package nulluse;

import java.util.List;

/** Uses of fields that hold null until assigned where Java prints, computes with or hands on null. */
public class Tag {
    private String text;
    private Integer count;
    private Shade shade;
    private List<String> words;

    public void set(String t, int n, Shade s, List<String> w) {
        this.text = t;
        this.count = n;
        this.shade = s;
        this.words = w;
    }

    public String formatted() {
        return String.format("%s", text);
    }

    public int doubled() {
        return count * 2;
    }

    public boolean isThree() {
        return count == 3;
    }

    public void bump() {
        count += 1;
    }

    public void step() {
        count++;
    }

    public int negated() {
        return -count;
    }

    public boolean sameAs(String other) {
        return other.equals(text);
    }

    public boolean isDark() {
        return shade == Shade.DARK;
    }

    public void addWord(String w) {
        words.add(w);
    }

    /** Java throws on a null text here, as the written program stops at the dereference: read. */
    public boolean isHello() {
        return text.equals("hello");
    }
}

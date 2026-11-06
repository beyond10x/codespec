package twoctors;

/**
 * A field assigned in only one of two constructors. The reader refuses the second constructor
 * (overloading); a call of it is refused, not read against the first one's parameters.
 */
public class Card {
    private String title;

    public Card() {
    }

    public Card(String t) {
        this.title = t;
    }

    public static boolean titled(String t) {
        return new Card(t).title != null;
    }
}

package nullescape;

/**
 * Reads of a field that holds null until assigned where Java goes on with the null: a getter returns
 * it, a package method takes it and ignores it, a local holds it and is overwritten. Java returns
 * null, "shelved" and "kept" on a fresh card.
 */
public class Card {
    private String title;

    public String title() {
        return title;
    }

    static String shelved(String t) {
        return "shelved";
    }

    public static String freshTitle() {
        return new Card().title();
    }

    public static String shelveFresh() {
        return shelved(new Card().title);
    }

    public static String keptAfterLocal() {
        Card c = new Card();
        String t = c.title;
        t = "kept";
        return t;
    }
}

package adversary

class Card {

    int copies = 3

    Object getProperty(String name) {
        return 99
    }

    static int peek() {
        Card c = new Card()
        return c.copies // diverges
    }
}

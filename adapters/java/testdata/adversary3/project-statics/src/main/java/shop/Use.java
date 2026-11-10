package shop;

import static shop.Fruit.deleteAll;

public class Use {
    /** javac calls Fruit.deleteAll(): the rows of Fruit. */
    public static long imported() {
        return deleteAll();
    }
}

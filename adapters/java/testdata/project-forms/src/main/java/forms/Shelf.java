package forms;

public class Shelf {
    public boolean named(Item item) {
        return item.name != null; // refused: Panache
    }

    public boolean stored(Item item) {
        return item.id != null;
    }
}

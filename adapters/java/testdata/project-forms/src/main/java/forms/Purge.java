package forms;

public class Purge {
    public long purge() {
        return Item.deleteAll() + Keyed.deleteAll();
    }
}

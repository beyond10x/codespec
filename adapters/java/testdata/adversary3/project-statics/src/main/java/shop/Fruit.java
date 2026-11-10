package shop;

import io.quarkus.hibernate.orm.panache.PanacheEntity;
import jakarta.persistence.Entity;

@Entity
public class Fruit extends PanacheEntity {
    public String name;

    /** javac calls Fruit.count(): the rows of Fruit. */
    public static long all() {
        return count();
    }

    public static class Helper {
        public long size;

        /** javac calls PanacheEntityBase.deleteAll(), which Panache does not enhance: it throws. */
        public static long purge() {
            return deleteAll();
        }
    }
}

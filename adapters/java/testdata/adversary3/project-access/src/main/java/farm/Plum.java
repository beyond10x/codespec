package farm;

import io.quarkus.hibernate.orm.panache.PanacheEntity;
import jakarta.persistence.Entity;

/** getSize(long) is not the getter: Quarkus generates getSize() and p.size calls that, a plain read. */
@Entity
public class Plum extends PanacheEntity {
    public long size;

    public long getSize(long factor) {
        return size * factor;
    }
}

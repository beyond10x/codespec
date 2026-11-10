package farm;

import jakarta.persistence.Entity;
import jakarta.persistence.Id;

/** A plain JPA entity: Quarkus rewrites every outside access of a public field into its getter. */
@Entity
public class Crop {
    @Id
    public long id;
    public long yield;

    public long getYield() {
        return yield * 10;
    }
}

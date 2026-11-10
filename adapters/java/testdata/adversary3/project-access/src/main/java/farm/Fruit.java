package farm;

import io.quarkus.hibernate.orm.panache.PanacheEntity;
import jakarta.persistence.Entity;

/** Quarkus rewrites the access in heavier (any method but the getter itself) into getWeight(). */
@Entity
public class Fruit extends PanacheEntity {
    public long weight;

    public long getWeight() {
        return weight * 2;
    }

    public long heavier(long by) {
        return weight + by;
    }
}

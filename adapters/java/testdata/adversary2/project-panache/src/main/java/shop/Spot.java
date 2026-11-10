package shop;

import io.quarkus.hibernate.orm.panache.PanacheEntity;
import jakarta.persistence.Entity;

/** Quarkus names the getter of xCoord getxCoord (JavaBeanUtil.capitalize keeps a name whose second letter is upper case). */
@Entity
public class Spot extends PanacheEntity {
    public long xCoord;

    public long getxCoord() {
        return 42;
    }
}

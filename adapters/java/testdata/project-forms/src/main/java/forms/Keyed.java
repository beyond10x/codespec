package forms;

import io.quarkus.hibernate.orm.panache.PanacheEntityBase;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;

@Entity
public class Keyed extends PanacheEntityBase {
    @Id
    public Long key;
}

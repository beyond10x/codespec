package forms;

import io.quarkus.hibernate.orm.panache.PanacheEntity;

public class Tag extends PanacheEntity { // refused: hides
    public Long id;
}

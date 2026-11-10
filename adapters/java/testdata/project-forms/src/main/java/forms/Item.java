package forms;

import io.quarkus.hibernate.orm.panache.PanacheEntity;

public class Item extends PanacheEntity {
    public String name;

    public String getName() {
        return "item";
    }
}

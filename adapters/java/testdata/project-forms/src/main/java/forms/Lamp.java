package forms;

import io.quarkus.hibernate.orm.panache.PanacheEntity;

public class Lamp extends PanacheEntity {
    public boolean lit;
    public boolean dim;

    public boolean isLit() {
        return true;
    }

    public boolean getDim() {
        return true;
    }
}

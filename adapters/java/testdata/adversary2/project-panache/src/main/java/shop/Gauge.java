package shop;

import io.quarkus.hibernate.orm.panache.PanacheEntity;
import jakarta.persistence.Entity;

@Entity
public class Gauge extends PanacheEntity {
    public long level;

    public void setLevel(long level) {
        this.level = level * 2;
    }
}

package shop;

import io.quarkus.hibernate.orm.panache.PanacheEntity;
import jakarta.persistence.Column;
import jakarta.persistence.Entity;

@Entity
public class Note extends PanacheEntity {
    @Column(name =
        "body_text")
    public String body;
}

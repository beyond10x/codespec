package forms;

import jakarta.persistence.Entity;

@Entity // refused: annotation jakarta.persistence.Entity
public interface Shape {
    long area();
}

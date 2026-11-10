package shop;

import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;

public class Marks {
    @Column // refused: annotation jakarta.persistence.Column
    public static final String LABEL = "x";

    @Id // refused: annotation jakarta.persistence.Id
    public long key() {
        return 1;
    }

    @Entity
    public static class Oops extends Exception { // refused: Entity
        private static final long serialVersionUID = 1L;
    }
}

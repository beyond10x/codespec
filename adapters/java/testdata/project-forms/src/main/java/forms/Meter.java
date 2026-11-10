package forms;

import jakarta.persistence.Entity;
import jakarta.persistence.Id;
import jakarta.persistence.Transient;

@Entity
public class Meter {
    @Id
    public Long id;
    public long v;
    public long copy;
    @Transient // refused: annotation with behaviour
    public long t;
    long p;

    public Meter() {
        v = 1;
        copy = v;
    }

    public long getV() {
        return v * 10;
    }

    public long getT() {
        return 55;
    }

    public long getP() {
        return p * 10;
    }
}

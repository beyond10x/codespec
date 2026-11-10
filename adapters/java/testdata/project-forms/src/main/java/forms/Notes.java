package forms;

import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;

@Entity
public class Notes {
    @Id
    public Long id;

    @Column(name = /* was "x" */ "a"   +   '"' + "b  c",
        length = // at most
            40)
    public String text;
}

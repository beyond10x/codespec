package marks;

import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;

@Entity
public class Row {
    @Id
    public Long id;

    @Column(name = // the legacy column
        "row_label")
    public String label;

    @Column(name = /* " */ "two  spaces")
    public String spaced;

    @Column(columnDefinition = """
        varchar(10)""")
    public String block;
}

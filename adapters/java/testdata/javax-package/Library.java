package javax.lending;

import java.util.ArrayList;
import java.util.List;

public class Library {
    private final List<Book> books = new ArrayList<>();
    private Genre genre = Genre.FICTION;

    public int count() {
        return books.size();
    }

    public void add(Book b) {
        books.add(b);
    }

    public Genre genre() {
        return genre;
    }

    public long total(Catalog c) {
        return c.size();
    }

    public void lend(int n) throws LendingException {
        if (n < 0) {
            throw new Overdue();
        }
    }
}

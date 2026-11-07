package adversary

import groovy.transform.CompileStatic

@CompileStatic
class Book {

    int copies = 0

    void setCopies(int n) {
        this.copies = n * 2
    }

    static int stock() {
        Book b = new Book()
        b.copies = 5 // diverges
        return b.copies
    }

    static int bump() {
        Book b = new Book()
        b.copies++ // diverges
        return b.copies
    }
}

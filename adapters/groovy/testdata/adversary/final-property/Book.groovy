package adversary

class Book {

    final int copies = 1

    static int restock() {
        Book b = new Book()
        b.copies = 7 // diverges
        return b.copies
    }
}

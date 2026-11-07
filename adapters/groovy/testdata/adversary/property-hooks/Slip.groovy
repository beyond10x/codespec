package adversary

class Slip {

    int copies = 0

    void setProperty(String name, Object value) {
    }

    static int poke() {
        Slip s = new Slip()
        s.copies = 7 // diverges
        return s.copies
    }
}

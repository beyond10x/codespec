package adversary

class Desk {

    static String grow() {
        def x = Integer.MAX_VALUE
        x += 1 // diverges
        x += 1L
        return "${x}"
    }
}

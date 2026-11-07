package adversary

class Desk {

    static boolean sameTitle(String title) {
        return title.equals("${title}") // diverges
    }
}

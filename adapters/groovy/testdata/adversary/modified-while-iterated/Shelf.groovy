package adversary

class Shelf {

    List<String> titles = new ArrayList<>()

    void add(String title) {
        titles.add(title)
    }

    int doubleUp() {
        for (t in titles) { // diverges
            add(t) // diverges
        }
        return titles.size()
    }
}

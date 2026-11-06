package typed

class Tally {

    long total = 0L
    int counter = 0
    List<String> names = new ArrayList<>()
    HashMap<String, Integer> counts = new HashMap<>()
    TreeMap<String, Integer> sorted = new TreeMap<>()

    void add(String name, long amount) {
        names.add(name)
        counts.put(name, counter)
        sorted.put(name, counter)
        total = total + amount
        counter++
    }

    int countOf(String wanted) {
        int n = 0
        for (s in names) {
            if (s == wanted) {
                n++
            }
        }
        return n
    }

    String unorderedKeys() {
        String out = ""
        for (String k : counts.keySet()) {
            out = out + k + ","
        }
        return out
    }

    String sortedKeys() {
        String out = ""
        for (String k : sorted.keySet()) {
            out = out + k + "=" + sorted.getOrDefault(k, 0) + ";"
        }
        return out
    }

    boolean over(long limit) {
        return total > limit && counter > 0
    }
}

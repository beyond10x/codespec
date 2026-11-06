package refuse

class Desk {

    int count(List<String> items) {
        int n = 0
        items.each { String s -> // refused closure
            n++
        }
        return n
    }
}

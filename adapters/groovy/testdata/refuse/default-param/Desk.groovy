package refuse

class Desk {

    int scaled(int n, int factor = 2) { // refused groovy-semantics
        return n * factor
    }

    int one() {
        return 1
    }
}

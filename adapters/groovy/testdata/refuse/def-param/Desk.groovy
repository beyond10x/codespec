package refuse

class Desk {

    String describe(def item) {
        return item.label() // refused dynamic
    }

    int one() {
        return 1
    }
}

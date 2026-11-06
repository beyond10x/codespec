package refuse

class Desk {

    def methodMissing(String name, args) { // refused dynamic
        return null
    }

    int one() {
        return 1
    }
}

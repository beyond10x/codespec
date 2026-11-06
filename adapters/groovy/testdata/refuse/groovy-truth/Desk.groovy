package refuse

class Desk {

    String state(List<String> items) {
        if (items) { // refused groovy-semantics
            return "some"
        }
        return "none"
    }
}

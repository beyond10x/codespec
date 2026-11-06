package refuse

class Desk {

    String share(int cents, int parts) {
        return "share " + (cents / parts) // refused groovy-semantics
    }
}

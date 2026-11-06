package refuse

class Desk {

    long share(int cents, long parts) {
        return cents.intdiv(parts) // refused groovy-semantics
    }
}

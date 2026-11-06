package refuse

class Desk {

    boolean even(short cents, short parts) {
        return cents.intdiv(parts) == 0 // refused groovy-semantics
    }
}

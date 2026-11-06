package refuse

class Box {

    int size = 0
}

class Desk {

    int sizeOf(Box box) {
        return box?.size // refused groovy-semantics
    }
}

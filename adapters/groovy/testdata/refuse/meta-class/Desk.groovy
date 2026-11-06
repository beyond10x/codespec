package refuse

class Box {

    int size = 0
}

class Desk {

    void install() {
        Box.metaClass.extra = "x" // refused dynamic
    }
}

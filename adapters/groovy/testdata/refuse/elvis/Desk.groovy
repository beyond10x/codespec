package refuse

class Desk {

    String nameOr(String name, String fallback) {
        return name ?: fallback // refused groovy-semantics
    }
}

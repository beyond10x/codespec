package typed

import groovy.transform.CompileStatic

@CompileStatic
class Label {

    static String describe(String name, int count) {
        return "${name} has ${count}"
    }

    static int sizeOf(Kind kind) {
        switch (kind) {
            case Kind.SMALL:
                return 1
            case Kind.LARGE:
                return 10
        }
        return 0
    }

    static int doubled(int n) {
        def twice = n * 2
        return twice
    }

    static int firstOver(List<Integer> xs, int limit) {
        int i = 0
        while (i < xs.size()) {
            if (xs.get(i) > limit) {
                break
            }
            i++
        }
        return i
    }

    static int wrap() {
        return Integer.MAX_VALUE + 1
    }
}

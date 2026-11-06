package refuse

import groovy.transform.CompileStatic

@CompileStatic
class Desk {

    long share(long cents, long parts) {
        long q = cents.intdiv(parts) // refused dynamic
        return q
    }
}

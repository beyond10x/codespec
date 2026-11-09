/* The C source of the kit case stmt-switch. */

/* Names a count. */
const char *name(long n) {
    switch (n) {
    case 1:
        return "one";
    case 2:
    case 3:
        return "few";
    default:
        break;
    }
    return "many";
}

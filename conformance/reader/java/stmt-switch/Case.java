package stmt_switch;

/** The Java source of the kit case stmt-switch; Java switches on an int, not a long. */
public final class Case {
    /** Names a count. */
    public static String name(int n) {
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
}

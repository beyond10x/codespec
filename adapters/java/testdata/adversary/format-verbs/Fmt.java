package formatverbs;

import java.util.ArrayList;
import java.util.List;

/** java.util.Formatter and cir/1 sprintf (Go fmt) print these differently. */
public class Fmt {
    /** Java "5"; Go "%!s(int32=5)". */
    public static String intAsS(int n) {
        return String.format("%s", n);
    }

    /** Java "ffffffff" for -1 (two's complement); Go "-1". */
    public static String hex(int n) {
        return String.format("%x", n);
    }

    /** Java "a" + line separator + "b"; Go "a%!n(MISSING)b". */
    public static String newline() {
        return String.format("a%nb");
    }

    /** Java "v v"; Go "%!$(string=v)s %!$(MISSING)s". */
    public static String indexed(String s) {
        return String.format("%1$s %1$s", s);
    }

    /** Java "[a, b]"; Go "[a b]". */
    public static String list() {
        List<String> l = new ArrayList<>();
        l.add("a");
        l.add("b");
        return String.format("%s", l);
    }

    /** The format is the caller's: neither its verbs nor their arguments are known. */
    public static String given(String f, int n) {
        return String.format(f, n);
    }
}

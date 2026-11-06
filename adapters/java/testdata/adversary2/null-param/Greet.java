package nullparam;

/** A String or boxed parameter may be null; Java prints it as "null", a cir/1 string or int has no null. */
public class Greet {
    /** Java: hello(null) is "hi null". */
    public static String hello(String name) {
        return "hi " + name;
    }

    /** Java: count(null) is "null items". */
    public static String count(Integer n) {
        return String.format("%d items", n);
    }
}

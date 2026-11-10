package b.lib;

public class Outer {
    public long w;

    /** Named as the top-level In of b.lib: b.lib refuses it and declares no type for it. */
    public static class In {
        public long v;
    }
}

package shop;

import org.jboss.logging.Logger;

public class Logs {
    private static final Logger NESTED = Logger.getLogger(Logs.Part.class.getName());
    private static final long STARTED = System.currentTimeMillis(); // refused: a static field initialized other than
    private static final String HOST = Logger.getLogger("host").getName(); // refused: a static field initialized other than

    public static class Part {
        public long n;
    }

    public boolean on() {
        return NESTED.isDebugEnabled();
    }
}

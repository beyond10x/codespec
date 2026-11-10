package forms;

public class Nesting {
    long n;

    public static class Inner { // refused: a nested class named as another type
        long n;
    }

    public static class Level {
        long n;

        public static class Deep { // refused: a class nested in a nested class
            long n;
        }
    }

    public interface Shape { // refused: nested interface
        long area();
    }
}

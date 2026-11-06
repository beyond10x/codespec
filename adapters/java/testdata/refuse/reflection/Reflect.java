package reflection;

public class Reflect {
    public String named() throws ClassNotFoundException {
        String n = Class.forName("reflection.Reflect").getName(); // refused
        return n;
    }
}

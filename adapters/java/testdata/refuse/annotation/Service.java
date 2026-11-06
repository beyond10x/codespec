package annotation;

public class Service {
    @Audited
    public int run() {
        return 1;
    }

    public boolean audited() throws NoSuchMethodException {
        boolean a = Service.class.getMethod("run").isAnnotationPresent(Audited.class); // refused
        return a;
    }
}

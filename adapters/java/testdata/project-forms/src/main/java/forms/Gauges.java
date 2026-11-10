package forms;

public class Gauges {
    public long transientField(Meter m) {
        return m.t;
    }

    public long packageField(Meter m) {
        return m.p;
    }

    public long read(Meter m) {
        return m.v; // refused: managed class
    }
}

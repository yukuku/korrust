package stringify;
public final class Probe {
    public static int gets;
    public int calls;
    public final int mode;
    public Probe(int mode) { this.mode = mode; }
    public static Probe next(int mode) { gets++; return mode == 0 ? null : new Probe(mode); }
    @Override public String toString() {
        calls++;
        if (mode == 2) return null;
        if (mode == 3) throw new IllegalStateException("stringify");
        return "probe";
    }
}

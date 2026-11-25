package anonymousjava;
public abstract class JavaBase {
    private final int seed;
    private final int initial;
    protected JavaBase(int seed) { this.seed = seed; initial = compute(2); }
    public int initial() { return initial; }
    public int compute(int value) { return seed + value; }
    protected int protectedValue() { return seed; }
    public abstract int read();
}

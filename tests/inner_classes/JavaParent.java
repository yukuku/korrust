package innerprobe;
public abstract class JavaParent {
    public final int during;
    protected JavaParent(int base) { during = base + read(); }
    public abstract int read();
}

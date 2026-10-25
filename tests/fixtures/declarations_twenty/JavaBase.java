package declarations.support;
public class JavaBase {
    protected final int bias;
    public JavaBase(int value) { bias = value; }
    protected int protectedValue() { return bias + 1; }
    public int compute() { return protectedValue(); }
}

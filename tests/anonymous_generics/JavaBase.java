package anonymousgeneric;
public class JavaBase<T> {
    public final T seed;
    public JavaBase(T seed) { this.seed = seed; }
    public T apply(T value) { return value; }
}

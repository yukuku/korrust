package fixture;
public abstract class Controller<C extends Controller<C,T>, T> {
    private final T value;
    protected Controller(T value) { this.value = value; }
    public T get() { return value; }
    public abstract C setup();
    public static <U extends V,V extends CharSequence> U echo(U value, V ignored) { return value; }
}

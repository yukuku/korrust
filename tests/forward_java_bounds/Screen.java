package fixture;
public final class Screen<T> extends Controller<Screen<T>,T> {
    public Screen(T value) { super(value); }
    public Screen<T> setup() { return this; }
}

package genericfields;
public final class NumberCell<T extends Number> {
    public T value;
    public NumberCell(T value) { this.value = value; }
}

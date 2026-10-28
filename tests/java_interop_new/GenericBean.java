package newinterop;

public class GenericBean<T> {
    private T value;
    public GenericBean(T value) { this.value = value; }
    public T getValue() { return value; }
    public void setValue(T value) { this.value = value; }
}

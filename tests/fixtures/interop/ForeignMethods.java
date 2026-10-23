package fixtures;
public class ForeignMethods {
    public static int staticValue() { return 1; }
    private int hidden() { return 1; }
    public int overloaded(int value) { return value; }
    public int overloaded(boolean value) { return value ? 1 : 0; }
    public <T> T generic(T value) { return value; }
    @Deprecated public int annotated() { return 1; }
    public int field = 1;
}

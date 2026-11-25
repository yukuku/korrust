public class ShortVarargCheck {
    public static void main(String[] args) {
        if (Short_varargKt.boundary() != 0 ||
            Short_varargKt.boundary(Short.MIN_VALUE, Short.MAX_VALUE) != 2)
            throw new AssertionError("Short vararg length");
    }
}

public class Caller {
    public static void main(String[] args) throws Exception {
        if (new Factory().value() != 12 || new Factory.Nested().value() != 11)
            throw new AssertionError();
        if (!java.lang.reflect.Modifier.isPrivate(
                Factory.Companion.class.getDeclaredMethod("secret", int.class).getModifiers()))
            throw new AssertionError();
    }
}

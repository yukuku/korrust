import anonymousgeneric.*;
public class JavaCheck {
    @SuppressWarnings({"rawtypes", "unchecked"})
    public static void main(String[] args) {
        JavaBase<String> result = JavaKt.javaStrings("!");
        if (!result.seed.equals("!") || !result.apply("x").equals("x!")) throw new AssertionError("Java superclass substitution");
        if (!((Mapper<String>) result).map("y").equals("y!")) throw new AssertionError("Kotlin interface");
        try { ((JavaBase) result).apply(2); throw new AssertionError("bridge cast"); } catch (ClassCastException expected) {}
        if (!result.getClass().getGenericSuperclass().getTypeName().equals("anonymousgeneric.JavaBase<java.lang.String>")) throw new AssertionError("Java signature");
        JavaBase<String> nullable = JavaKt.javaNullable(null);
        if (nullable.seed != null || nullable.apply(null) != null || !nullable.apply("x").equals("x")) throw new AssertionError("nullable Java platform overrides");
        System.out.println("OK");
    }
}

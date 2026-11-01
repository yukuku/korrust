package test;
public final class OriginalCheck {
    public static void main(String[] args) {
        if (StaticNestedClassKt.answer() != 42) throw new AssertionError("initial field");
        StaticNestedClass.StaticNested.ULTIMATE_QUESTION = 73;
        if (StaticNestedClassKt.answer() != 73) throw new AssertionError("mutable field");
        System.out.println("OK 2");
    }
}

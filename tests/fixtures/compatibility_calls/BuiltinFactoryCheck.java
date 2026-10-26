public final class BuiltinFactoryCheck {
    public static void main(String[] args) {
        String actual = compatibility.calls.factories.BuiltinFactoriesKt.builtinFactories();
        if (!actual.equals("2:2:3:4:10:8")) throw new AssertionError(actual);
        System.out.println(actual);
    }
}

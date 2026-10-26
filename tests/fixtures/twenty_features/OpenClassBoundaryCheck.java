public final class OpenClassBoundaryCheck {
    public static final class Derived extends E {
        public Derived(int value) { super(value); }
    }

    public static void main(String[] args) throws Exception {
        if (new E(-7).getValue() != -7) throw new AssertionError("constructor property");
        if (new Derived(19).getValue() != 19) throw new AssertionError("subclass");
        if (java.lang.reflect.Modifier.isFinal(E.class.getModifiers())) throw new AssertionError("open class");
        if (!java.lang.reflect.Modifier.isPublic(E.class.getDeclaredConstructor(int.class).getModifiers())) throw new AssertionError("constructor");
        System.out.println("{\"id\":\"general_inheritance\",\"checks\":4}");
    }
}

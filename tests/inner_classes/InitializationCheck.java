import innerprobe.*;
public final class InitializationCheck {
    private static void eq(int expected, int actual) { if (expected != actual) throw new AssertionError(expected + " != " + actual); }
    public static void main(String[] args) throws Exception {
        Initialization outer = new Initialization(6);
        Initialization.Child child = outer.make(4);
        eq(12, child.during); eq(10, child.getInitialized()); eq(10, child.read());
        outer.setValue(10); eq(14, child.read()); eq(10, child.getInitialized());
        Shadow.Item item = new Shadow(20).new Item(3); eq(23, item.sum()); eq(27, item.local(4));
        InitializationKt.setOrder(0); eq(105, InitializationKt.ordered()); eq(123, InitializationKt.getOrder());
        InitializationKt.setOrder(0);
        try { InitializationKt.failsBeforeArguments(); throw new AssertionError("expected null failure"); } catch (NullPointerException expected) {}
        eq(1, InitializationKt.getOrder());
        InitializationKt.setOrder(0);
        if (InitializationKt.safeFactory(null) != null) throw new AssertionError("safe null");
        eq(0, InitializationKt.getOrder());
        PairOuter pairOuter = new PairOuter(8);
        eq(13, InitializationKt.safeFactory(pairOuter).sum()); eq(23, InitializationKt.getOrder());
        eq(13, InitializationKt.bound(pairOuter).invoke(2, 3).sum());
        eq(15, InitializationKt.unbound().invoke(pairOuter, 3, 4).sum());
        ReferenceOuter.Entry reference = new ReferenceOuter(null).new Entry("x", 4L, 2.5d, null);
        if (!reference.text().equals("none:x:null") || reference.score() != 6.5d) throw new AssertionError("reference and wide constructor parameters");
        Class<?> type = Initialization.Child.class;
        if (type.getDeclaringClass() != Initialization.class || java.lang.reflect.Modifier.isStatic(type.getModifiers())) throw new AssertionError("inner nesting");
        var field = type.getDeclaredField("this$0");
        if (!field.isSynthetic() || !java.lang.reflect.Modifier.isFinal(field.getModifiers())) throw new AssertionError("capture field");
        if (type.getDeclaredConstructors()[0].getParameterTypes()[0] != Initialization.class) throw new AssertionError("enclosing parameter");
    }
}

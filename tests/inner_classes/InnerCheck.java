import innerprobe.Outer;
public final class InnerCheck {
    private static void eq(int expected, int actual) { if (expected != actual) throw new AssertionError(expected + " != " + actual); }
    public static void main(String[] args) throws Exception {
        eq(26, innerconsumer.ConsumerKt.consume());
        innerprobe.GenericArgumentsOuter generic = new innerprobe.GenericArgumentsOuter();
        innerprobe.GenericArgumentsOuter.Entry entry = generic.new Entry(java.util.List.of("OK"));
        if (!entry.getValues().equals(java.util.List.of("OK"))) throw new AssertionError("generic constructor value");
        var constructor = innerprobe.GenericArgumentsOuter.Entry.class.getConstructors()[0];
        eq(2, constructor.getParameterCount());
        eq(1, constructor.getGenericParameterTypes().length);
        if (!constructor.getGenericParameterTypes()[0].getTypeName().equals("java.util.List<java.lang.String>")) throw new AssertionError("generic constructor signature");
        Outer outer = new Outer(10);
        Outer.Inner first = outer.new Inner(2);
        Outer.Inner second = outer.make(5);
        Outer.Inner third = innerprobe.OuterKt.qualified(outer, 3);
        eq(12, first.getAtInit()); eq(15, second.read()); eq(13, third.read());
        if (first.outer() != outer) throw new AssertionError("outer identity");
        kotlin.jvm.functions.Function0<Integer> callback = first.callback();
        eq(20, first.update(20)); eq(25, second.read()); eq(22, callback.invoke());
        eq(9, first.reveal()); eq(9, second.readSecret()); eq(14, second.reveal()); eq(14, first.readSecret());
        Outer other = new Outer(100); eq(103, other.new Inner(3).read()); eq(22, first.read());
    }
}

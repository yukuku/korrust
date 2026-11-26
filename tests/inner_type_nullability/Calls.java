import nestedapi.*;
public final class Calls {
    private static void check(boolean value) { if (!value) throw new AssertionError(); }
    public static void main(String[] args) {
        check(EffectsKt.nullable(true) == null);
        check(EffectsKt.nullable(false).value == 4);
        check(EffectsKt.nonNull().value == 6);
        check(EffectsKt.nested(true) == null && EffectsKt.nested(false).value == 8);
        check(EffectsKt.acceptNull() == -1 && EffectsKt.accept(null) == -1);
        check(EffectsKt.accept(new Outer.Leaf(9)) == 9);
        check(EffectsKt.value(true) == -1 && EffectsKt.value(false) == 4);
        Provider provider = new Provider();
        check(EffectsKt.field(provider) == null);
        Outer.Leaf leaf = new Outer.Leaf(11);
        EffectsKt.write(provider, leaf); check(EffectsKt.field(provider) == leaf);
        EffectsKt.write(provider, null); check(EffectsKt.field(provider) == null);
        check(EffectsKt.staticField() == null);
        Outer.Middle.Leaf nested = new Outer.Middle.Leaf(13);
        EffectsKt.writeStatic(nested); check(EffectsKt.staticField() == nested);
        EffectsKt.writeStatic(null); check(EffectsKt.staticField() == null);
        System.out.println("OK");
    }
}

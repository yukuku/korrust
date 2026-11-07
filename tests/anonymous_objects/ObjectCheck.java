import anonymousprobe.*;
public class ObjectCheck {
    public static void main(String[] args) {
        Counter a = ObjectsKt.counter(10);
        Counter b = ObjectsKt.counter(10);
        if (a == b || a.self() != a || b.self() != b) throw new AssertionError("identity");
        if (a.add(5) != 15 || a.current() != 15 || b.current() != 10 || a.doubled() != 30) throw new AssertionError("independent cells");
        if (ObjectsKt.shared() != 710) throw new AssertionError("shared capture");
        Counter c = new Owner(20).make(3);
        if (c.current() != 23 || c.add(2) != 25 || c.self() != c) throw new AssertionError("outer and own this");
        if (ObjectsKt.immutable(9).add(2) != 20 || ObjectsKt.immutable(9).current() != 18) throw new AssertionError("immutable capture");
        if (ObjectsKt.zero() == ObjectsKt.zero()) throw new AssertionError("fresh zero capture");
        if (ObjectsKt.nested() != 13 || ObjectsKt.siblings() != 78) throw new AssertionError("nested and sibling closures");
        if (ObjectsKt.shadow(90).current() != 4) throw new AssertionError("parameter shadowing");
        if (ObjectsKt.effects() != 2007) throw new AssertionError("effects and delayed invocation");
        if (new FactoryOwner(3).make().current() != 8) throw new AssertionError("outer method and private field");
        if (ObjectsKt.wide(5L, 2.5, "x").read(2L) != 9L) throw new AssertionError("wide capture");
        if (new CollisionOwner(3).make().current() != 9) throw new AssertionError("capture field collision");
        if (new PrecedenceOwner(3).make().add(2) != 5) throw new AssertionError("own method precedence");
        Counter shadow = ObjectsKt.mixedShadow(90);
        if (shadow.add(4) != 4 || shadow.current() != 90) throw new AssertionError("cross-method parameter shadow");
        Counter local = ObjectsKt.mixedLocal(90);
        if (local.add(2) != 6 || local.current() != 90) throw new AssertionError("cross-method local shadow");
        if (ObjectsKt.namedValue().add(1) != 10 || ObjectsKt.namedCallable().add(1) != 8) throw new AssertionError("function and value namespaces");
        if (ObjectsKt.lambdaShadow(7) != 11) throw new AssertionError("ordinary lambda local shadow");
        System.out.println("OK");
    }
}

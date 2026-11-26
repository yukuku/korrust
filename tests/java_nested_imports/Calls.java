package nested.consumer;
import nested.library.Outer;
public class Calls {
    public static void main(String[] args) {
        if (ConsumerKt.dollar() != 99) throw new AssertionError();
        if (ConsumerKt.value() != 5) throw new AssertionError();
        if (ConsumerKt.kind() != Outer.Kind.B) throw new AssertionError();
        if (!ConsumerKt.leaf().equals("leaf:leaf")) throw new AssertionError();
        if (!ConsumerKt.unbox(() -> "box").equals("box")) throw new AssertionError();
        if (ConsumerKt.inner(new Outer().new Inner(9)) != 9) throw new AssertionError();
        if (!ConsumerKt.shadow(() -> "shadow").equals("shadow")) throw new AssertionError();
        if (ConsumerKt.kotlinNested() != 4) throw new AssertionError();
        if (nested.library.PackageConsumerKt.samePackage() != 31) throw new AssertionError();
        System.out.println("java-nested-imports-ok");
    }
}

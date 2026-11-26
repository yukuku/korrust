import fixture.Holder.Status;
import usage.ConsumerKt;
public class Caller {
    public static void main(String[] args) {
        Status[] a = ConsumerKt.qualified();
        Status[] b = ConsumerKt.simple();
        if (a == b || a[0] != Status.FIRST || a[1] != Status.LAST || b[1] != a[1] || b.length != 2) throw new AssertionError();
        if (ConsumerKt.overload() != 7 || ConsumerKt.shadowed(new fixture.Ordinary()) != 19) throw new AssertionError();
    }
}

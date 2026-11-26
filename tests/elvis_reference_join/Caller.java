import sample.ConsumerKt;
import library.Types;
import library.Types.Base;
import library.Types.Leaf;
public class Caller {
    public static void main(String[] args) throws Exception {
        Base b = new Base();
        Leaf l = new Leaf();
        if (ConsumerKt.wider(l,b) != l || ConsumerKt.wider(null,b) != b ||
            ConsumerKt.narrower(b,l) != b || ConsumerKt.narrower(null,l) != l) throw new AssertionError();
        if (ConsumerKt.class.getDeclaredMethod("wider",Leaf.class,Base.class).getReturnType() != Base.class) throw new AssertionError();
        Types t = new Types();
        if (ConsumerKt.items(t) != 0 || ConsumerKt.items(null) != 0) throw new AssertionError();
        t.items = java.util.List.of("one");
        if (ConsumerKt.items(t) != 1) throw new AssertionError();
    }
}

package nestedliteralcheck;
import nestedliteral.Outer;
import nestedliteraluses.UsesKt;
public class Calls {
    public static void main(String[] args) {
        if (UsesKt.nested() != Outer.Nested.class || UsesKt.imported() != Outer.Nested.class) throw new AssertionError("nested");
        if (UsesKt.inner() != Outer.Inner.class || UsesKt.contract() != Outer.Contract.class || UsesKt.mode() != Outer.Mode.class) throw new AssertionError("class kinds");
        if (UsesKt.jdk() != java.util.Map.Entry.class) throw new AssertionError("JDK generic nested interface");
        System.out.println("java-nested-literals-ok");
    }
}

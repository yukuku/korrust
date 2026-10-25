import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_inheritance_super {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "inheritance_super": check(InheritanceKt.inherited(new Child(4)) == 7); check(InheritanceKt.inherited(new Parent(4)) == 5); check(InheritanceKt.abstractInherited(new ConcreteChild()) == 7); declarations.support.JavaBase javaBase = new JavaChild(4); check(javaBase.compute() == 12); check(((JavaChild) javaBase).protectedCapture() == 5); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

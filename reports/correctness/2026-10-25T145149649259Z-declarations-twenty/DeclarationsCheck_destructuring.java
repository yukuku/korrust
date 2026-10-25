import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_destructuring {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "destructuring": Components components = new Components(); check(DestructureKt.destructure(components) == 7); events("12"); Trace.clear(); check(DestructureKt.destructureSkip(components).equals("abc")); events("2"); Trace.clear(); check(DestructureKt.destructureLoop(java.util.Arrays.asList(components, components)) == 14); events("1212"); Trace.clear(); check(DestructureKt.destructureLambda(components) == 7); events("12"); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

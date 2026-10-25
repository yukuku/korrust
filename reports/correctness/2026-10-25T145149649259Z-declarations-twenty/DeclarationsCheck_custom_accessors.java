import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_custom_accessors {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "custom_accessors": check(AccessorsKt.accessorValue() == 13); events("21"); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

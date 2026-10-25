import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_secondary_constructors {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "secondary_constructors": check(SecondaryKt.secondary() == 8); events("12123"); check(new SecondaryCollision(5).getStored() == 12); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

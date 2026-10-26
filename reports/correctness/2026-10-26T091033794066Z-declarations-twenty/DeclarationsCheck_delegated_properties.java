import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_delegated_properties {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "delegated_properties": check(DelegatesKt.delegatedValue() == 8); events("321"); Trace.clear(); DelegatedBox d1 = new DelegatedBox(); DelegatedBox d2 = new DelegatedBox(); d1.setScore(7); check(d1.getScore() == 7); check(d2.getScore() == 3); check(d1.getScore() == 7); events("332111"); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_operator_contains {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "operator_contains": Bag<String> bag = new Bag<>("yes"); check(MembershipKt.membership(bag,"yes")); check(MembershipKt.absent(bag,"no")); check(MembershipKt.concreteMembership(7)); check(!MembershipKt.concreteMembership(8)); events("11"); Trace.clear(); check(MembershipKt.containsOrder()); events("213"); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

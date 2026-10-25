import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_operator_index {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "operator_index": Slots<String> slots = new Slots<>("old"); check(OperatorsKt.readSlot(slots,0).equals("old")); events("2"); Trace.clear(); OperatorsKt.writeSlot(slots,"new"); events("03"); check(slots.getValue().equals("new")); check(OperatorsKt.concreteSlots() == 10); try { OperatorsKt.readSlot(slots,1); throw new AssertionError("missing bounds"); } catch (IndexOutOfBoundsException e) { check(true); } Trace.clear(); check(OperatorsKt.multiSlot(slots).equals("multi")); events("1254"); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

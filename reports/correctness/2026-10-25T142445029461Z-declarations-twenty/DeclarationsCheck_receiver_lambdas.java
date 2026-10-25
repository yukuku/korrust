import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_receiver_lambdas {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "receiver_lambdas": check(ReceiversKt.receiverLambda().equals("a23")); StringBuilder receiver = new StringBuilder(); ReceiversKt.sourceReceiver(receiver, (builder, amount) -> { builder.append(amount); return kotlin.Unit.INSTANCE; }); check(receiver.toString().equals("4")); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

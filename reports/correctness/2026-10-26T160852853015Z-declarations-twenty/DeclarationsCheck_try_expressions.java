import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_try_expressions {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "try_expressions": check(TryFlowKt.tryValue(4) == 5); check(TryFlowKt.tryValue(0) == 7); check(TryFlowKt.tryValue(-1) == 8); events("111"); Trace.clear(); check(TryFlowKt.finallyReturn() == 2); events("23"); check(TryFlowKt.finallyOverride() == 9); Trace.clear(); try { TryFlowKt.finallyThrow(); throw new AssertionError("missing throw"); } catch (IllegalStateException e) { check(e.getMessage().equals("original")); } events("4"); Trace.clear(); check(TryFlowKt.finallyBypass() == 8); events(""); Trace.clear(); check(TryFlowKt.nestedCatchFinally() == 5); events("3"); Trace.clear(); RuntimeException original = new RuntimeException("identity"); try { TryFlowKt.rethrowSame(original); throw new AssertionError("missing rethrow"); } catch(RuntimeException caught) { check(caught == original); } events("8"); check(TryFlowKt.tryReference(false).compute() == 5); check(TryFlowKt.tryReference(true).compute() == 9); check(TryFlowKt.mutableReference(false).compute() == 5); check(TryFlowKt.mutableReference(true).compute() == 9); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

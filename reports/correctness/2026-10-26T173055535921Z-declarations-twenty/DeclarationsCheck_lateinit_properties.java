import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_lateinit_properties {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "lateinit_properties": check(LateinitKt.lateValue() == 5); try { new LateBox().getText(); throw new AssertionError("missing lateinit"); } catch (kotlin.UninitializedPropertyAccessException e) { check(e.getMessage().contains("text")); } LateBox late = new LateBox(); late.setText("ok"); check(late.getText().equals("ok")); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

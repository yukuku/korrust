import declarations.probes.*;
import declarations.library.*;
import declarations.support.Trace;
import java.util.Arrays;
public final class DeclarationsCheck_uniform_bodies {
    static int checks;
    static void check(boolean condition) { checks++; if (!condition) throw new AssertionError("check " + checks + " events=" + Trace.events); }
    static void events(String expected) { check(Trace.events.equals(expected)); }
    public static void main(String[] args) {
        Trace.clear();
        switch (args[0]) {
            case "uniform_bodies": check(BodiesKt.bodies() == 16); check(new BodyClass(3).getDoubled() == 6); check(new BodyClass(3).inferred() == 4); check(new BodyClass(3).weighted("abc", 1.5f) == 4.5f); check(BodyObject.INSTANCE.weighted("abc", 1.5f) == 4.5f); PrivateCapture capture = new PrivateCapture(7); check(capture.calc() == 8); check(capture.invokePrivate() == 8); break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

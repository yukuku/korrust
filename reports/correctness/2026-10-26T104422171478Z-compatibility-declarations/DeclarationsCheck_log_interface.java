import compatibility.declarations.*;
import java.lang.reflect.Method;
import java.util.Objects;
import kotlin.jvm.functions.Function1;
import kotlin.Unit;

public final class DeclarationsCheck_log_interface {
    static int checks;
    static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
            case "log_interface": {
                Effects.reset();
                eq(yuku.alkitab.datatransfer.process.LogInterface.class.getMethod("logger", String.class).isDefault(), true);
                eq(yuku.alkitab.datatransfer.process.LogInterface.class.getMethod("log", String.class).isDefault(), false);
                eq(java.lang.reflect.Modifier.isStatic(Class.forName("yuku.alkitab.datatransfer.process.LogInterface$DefaultImpls").getDeclaredMethod("logger", yuku.alkitab.datatransfer.process.LogInterface.class, String.class).getModifiers()), true);
                Function1<String, Unit> logger = LogCallsKt.appLogger("J");
                logger.invoke("one"); logger.invoke("two");
                eq(Effects.events, "[J] one;[J] two;");
                eq(LogCallsKt.appCapturedLog("C"), "[C] alpha;[C] beta;");
                break;
            }
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"id\":\"" + args[0] + "\",\"checks\":" + checks + "}");
    }
}

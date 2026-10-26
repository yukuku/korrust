import compatibility.declarations.*;
import java.lang.reflect.Method;
import java.util.Objects;
import kotlin.jvm.functions.Function1;
import kotlin.Unit;

public final class DeclarationsCheck_discards {
    static int checks;
    static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
            case "discards":
                eq(DiscardsKt.discardedLocal(), "12:I2");
                eq(DiscardsKt.discardedAll(), "I");
                eq(DiscardsKt.discardedParameters(), 10);
                eq(DiscardsKt.discardedLambda(), "33:I3");
                eq(DiscardsKt.repeatedDiscardParameters(), 11);
                eq(DiscardsKt.discardedLoop(), "9:II33");
                break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"id\":\"" + args[0] + "\",\"checks\":" + checks + "}");
    }
}

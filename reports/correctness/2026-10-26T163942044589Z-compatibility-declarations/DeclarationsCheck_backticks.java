import compatibility.declarations.*;
import java.lang.reflect.Method;
import java.util.Objects;
import kotlin.jvm.functions.Function1;
import kotlin.Unit;

public final class DeclarationsCheck_backticks {
    static int checks;
    static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
            case "backticks": {
                DescriptiveTest value = new DescriptiveTest();
                Effects.reset();
                Method test = DescriptiveTest.class.getDeclaredMethod("adds two numbers");
                eq(test.getAnnotation(org.junit.Test.class) != null, true);
                test.invoke(value);
                eq(Effects.events, "B");
                eq(DescriptiveTest.class.getDeclaredMethod("when", int.class).invoke(value, 8), 9);
                eq(value.plain(4), 6);
                eq(BackticksKt.escapedCall(20), 21);
                break;
            }
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"id\":\"" + args[0] + "\",\"checks\":" + checks + "}");
    }
}

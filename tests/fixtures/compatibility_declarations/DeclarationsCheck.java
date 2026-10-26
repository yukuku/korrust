import compatibility.declarations.*;
import java.lang.reflect.Method;
import java.util.Objects;
import kotlin.jvm.functions.Function1;
import kotlin.Unit;

public final class DeclarationsCheck {
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
            case "discards":
                eq(DiscardsKt.discardedLocal(), "12:I2");
                eq(DiscardsKt.discardedAll(), "I");
                eq(DiscardsKt.discardedParameters(), 10);
                eq(DiscardsKt.discardedLambda(), "33:I3");
                eq(DiscardsKt.repeatedDiscardParameters(), 11);
                eq(DiscardsKt.discardedLoop(), "9:II33");
                break;
            case "functional_interfaces": {
                eq(Mapper.class.getMethod("twice", int.class).isDefault(), true);
                eq(MapperParent.class.getMethod("apply", int.class).isDefault(), false);
                eq(java.lang.reflect.Modifier.isStatic(Class.forName("compatibility.declarations.Mapper$DefaultImpls").getDeclaredMethod("twice", Mapper.class, int.class).getModifiers()), true);
                Mapper mapper = FunctionalInterfacesKt.newMapper(7);
                eq(mapper.apply(3), 10);
                eq(mapper.twice(3), 20);
                Mapper captured = FunctionalInterfacesKt.capturedMapper(2);
                eq(captured.apply(3), 5);
                eq(captured.apply(4), 9);
                eq(captured.twice(1), 21);
                eq(captured.apply(0), 11);
                eq(FunctionalInterfacesKt.capturedSink("prefix"), "prefix:one;prefix:two;");
                eq(FunctionalInterfacesKt.distinguishTypes(), "sam:function");
                eq(FunctionalInterfacesKt.overloadChoices(), "6:11:16:16:FLSMFT");
                Effects.reset();
                eq(FunctionalInterfacesKt.dispatch(FunctionalInterfacesKt.newMapper(7)), 12);
                Function1<Integer, Integer> function = value -> value + 9;
                eq(FunctionalInterfacesKt.dispatch(function), 14);
                eq(Effects.events, "SF");
                break;
            }
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
            case "type_aliases":
                eq(AliasesKt.label("value"), "value");
                eq(AliasesKt.privateAliasBody("hidden"), "hidden");
                eq(AliasesKt.aliasApply(5, value -> value + 10), 15);
                eq(AliasUsesKt.chainedAlias("chain"), "chain");
                eq(AliasUsesKt.callbackAlias(4), 9);
                break;
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"id\":\"" + args[0] + "\",\"checks\":" + checks + "}");
    }
}

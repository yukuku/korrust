import compatibility.declarations.*;
import java.lang.reflect.Method;
import java.util.Objects;
import kotlin.jvm.functions.Function1;
import kotlin.Unit;

public final class DeclarationsCheck_functional_interfaces {
    static int checks;
    static void eq(Object actual, Object expected) {
        checks++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    public static void main(String[] args) throws Exception {
        switch (args[0]) {
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
            default: throw new AssertionError(args[0]);
        }
        System.out.println("{\"id\":\"" + args[0] + "\",\"checks\":" + checks + "}");
    }
}

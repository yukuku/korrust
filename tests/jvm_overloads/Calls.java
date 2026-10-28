package overloads;

public class Calls {
    public static void main(String[] args) {
        check("1:B:3", FunctionsKt.top("B"));
        check("8:B:3", FunctionsKt.top(8, "B"));
        check("8:B:9", FunctionsKt.top(8, "B", 9L));
        check("1:member", new Member().value());
        check("9:member", new Member().value(9));
        check("2:companion", Member.staticValue());
        check("9:companion", Member.Companion.staticValue(9));
        check(4L, Member.Companion.instance());
        check("5:object", Singleton.staticValue());
        check(6L, Singleton.INSTANCE.instance());
        check(39, new VirtualDerived().value());
        check(23, new Stateful(23).value());
        check(17, DefaultState.INSTANCE.value());
        check(19, CompanionState.Companion.value());
        check("T", FunctionsKt.generic("T"));
        check(2.5, FunctionsKt.nullable());
        check(3.5, FunctionsKt.nullable("a"));
        check("V:2", FunctionsKt.varargs(new int[] {1, 2}));
        check("X:1", FunctionsKt.varargs("X", 1));
        Effects.log = "";
        check("1:R:3", FunctionsKt.effects("R"));
        check("1;3;", Effects.log);
        Effects.log = "";
        check("9:R:11", FunctionsKt.effects(9, "R"));
        check("11;", Effects.log);
        Effects.log = "";
        try { FunctionsKt.effects((String) null); throw new AssertionError("null accepted"); }
        catch (NullPointerException expected) { check("", Effects.log); }
        try { FunctionsKt.throwing(7); throw new AssertionError("default did not throw"); }
        catch (IllegalStateException expected) { check("default", expected.getMessage()); check("fail;", Effects.log); }
        check(49, FunctionsKt.mask32(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31));
        try {
            check("method", FunctionsKt.class.getMethod("annotated", String.class).getAnnotation(Marker.class).value());
            check("required", ((Marker) FunctionsKt.class.getMethod("annotated", String.class).getParameterAnnotations()[0][0]).value());
            check("first", ((Marker) FunctionsKt.class.getMethod("annotated", int.class, String.class).getParameterAnnotations()[0][0]).value());
            check("required", ((Marker) FunctionsKt.class.getMethod("annotated", int.class, String.class).getParameterAnnotations()[1][0]).value());
            check("companion", Annotations.class.getMethod("marked", String.class).getAnnotation(Marker.class).value());
            check("required", ((Marker) Annotations.class.getMethod("marked", String.class).getParameterAnnotations()[0][0]).value());
            check("required", ((Marker) Annotations.Companion.getClass().getMethod("marked", String.class).getParameterAnnotations()[0][0]).value());
            check(IllegalStateException.class, Exceptional.class.getMethod("value").getExceptionTypes()[0]);
            check(true, Exceptional.class.getMethod("value").isAnnotationPresent(kotlin.jvm.JvmStatic.class));
        } catch (ReflectiveOperationException exception) { throw new AssertionError(exception); }
        System.out.println("OK");
    }

    private static void check(Object expected, Object actual) {
        if (!expected.equals(actual)) throw new AssertionError(expected + " != " + actual);
    }
}

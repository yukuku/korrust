import java.lang.reflect.*;
import java.util.*;

public final class Check {
    static int assertions;
    static void yes(boolean value) { assertions++; if (!value) throw new AssertionError("assertion " + assertions); }
    static void equal(Object actual, Object expected) { assertions++; if (!Objects.equals(actual, expected)) throw new AssertionError("assertion " + assertions + ": " + actual + " != " + expected); }
    static Class<?> cls(String name) throws Exception { return Class.forName("valuecases." + name); }
    static Object call(Class<?> owner, String name, Object... args) throws Exception {
        for (Method method : owner.getDeclaredMethods()) {
            if (method.getName().equals(name) && method.getParameterCount() == args.length) return method.invoke(null, args);
        }
        throw new NoSuchMethodException(owner + "." + name);
    }
    static Method top(String name) throws Exception {
        for (Method method : cls("ValuesKt").getDeclaredMethods()) {
            if (method.getName().equals(name) || method.getName().startsWith(name + "-")) {
                if (!method.isSynthetic()) return method;
            }
        }
        throw new NoSuchMethodException(name);
    }
    static Object invoke(String name, Object... args) throws Exception { return top(name).invoke(null, args); }
    static Object box(String name, Object raw) throws Exception { return call(cls(name), "box-impl", raw); }
    static Object unbox(Object value) throws Exception { return value.getClass().getDeclaredMethod("unbox-impl").invoke(value); }
    public static void main(String[] args) throws Exception {
        for (int raw : new int[] {0, 1, -1, Integer.MIN_VALUE, Integer.MAX_VALUE}) {
            Object value = box("Id", raw);
            equal(unbox(value), raw);
            equal(value.toString(), "Id(raw=" + raw + ")");
            equal(value.hashCode(), raw);
            equal(value, box("Id", raw));
            yes(!value.equals(box("Id", raw ^ 1)));
            yes(!value.equals(raw));
            yes(!value.equals(null));
            equal(call(cls("Id"), "equals-impl0", raw, raw), true);
            equal(call(cls("Id"), "equals-impl0", raw, raw ^ 1), false);
            equal(invoke("createId", raw), raw);
            equal(unbox(invoke("asAny", raw)), raw);
            equal(invoke("fromAny", value), raw);
            equal(invoke("nullableId", value), value);
        }
        Object secret=box("Secret",4);
        equal(unbox(secret),4);
        equal(call(cls("Secret"),"read-impl",4),4);
        Object capture=invoke("capture",11);
        Method function=capture.getClass().getMethod("invoke");
        function.setAccessible(true);
        equal(unbox(function.invoke(capture)),11);
        equal(invoke("nullableId", (Object) null), null);
        equal(top("nullableId").getParameterTypes()[0], cls("Id"));
        equal(top("nullableText").getParameterTypes()[0], String.class);
        equal(top("nullableMaybeText").getParameterTypes()[0], cls("MaybeText"));
        equal(invoke("nullableText", (Object) null), null);
        equal(invoke("nullableText", "text"), "text");
        Object absent = invoke("nullMaybe");
        Object present = invoke("boxedNullMaybe");
        equal(absent, null);
        yes(present != null);
        equal(unbox(present), null);
        equal(invoke("nullableMaybeText", present), present);
        equal(present.toString(), "MaybeText(raw=null)");
        for (String raw : new String[] {"", "abc", "\uD83D\uDE00", "\uD800", "\u0000"}) {
            Object value = box("Text", raw);
            equal(unbox(value), raw);
            equal(value.toString(), raw);
            equal(value.hashCode(), raw.hashCode());
            equal(value, box("Text", raw));
        }
        for (long raw : new long[] {0L, -1L, Long.MIN_VALUE, Long.MAX_VALUE}) {
            Object value = box("Wide", raw);
            equal(unbox(value), raw);
            equal(value.hashCode(), Long.hashCode(raw));
            equal(value.toString(), "Wide(raw=" + raw + ")");
            Object show = invoke("asShow", raw);
            equal(cls("Show").getMethod("show").invoke(show), "Wide" + raw);
            equal(invoke("fromShow", show), raw);
        }
        equal(cls("Holder").getDeclaredField("value").getType(), int.class);
        Object holder=cls("Holder").getConstructor(int.class,Class.forName("kotlin.jvm.internal.DefaultConstructorMarker")).newInstance(12,null);
        Object holderCapture=cls("Holder").getMethod("capture").invoke(holder);
        Method holderFunction=holderCapture.getClass().getMethod("invoke");
        holderFunction.setAccessible(true);
        equal(unbox(holderFunction.invoke(holderCapture)),12);
        equal(cls("Pair").getDeclaredField("id").getType(), int.class);
        equal(cls("Pair").getDeclaredField("wide").getType(), long.class);
        equal(cls("Pair").getDeclaredField("text").getType(), String.class);
        Object mixed=invoke("mixed",Arrays.asList("a","b"),null);
        equal(unbox(((List<?>)mixed).get(0)),2);
        equal(top("mixed").getParameterTypes()[1],cls("Id"));
        Object keeper=cls("Mixed").getConstructor().newInstance();
        Method keep=null;
        for (Method method:cls("Mixed").getDeclaredMethods()) if(method.getName().startsWith("keep-")&&!method.isSynthetic()) keep=method;
        yes(keep!=null);
        equal(keep.getParameterTypes()[1],cls("Id"));
        equal(keep.getReturnType(),cls("Id"));
        equal(keep.invoke(keeper,Arrays.asList("a"),null),null);
        Object generic = invoke("generic", 9);
        equal(unbox(cls("Box").getMethod("getValue").invoke(generic)), 9);
        Object list = invoke("list", 10);
        equal(unbox(((List<?>) list).get(0)), 10);
        Object effects = cls("Effects").getField("INSTANCE").get(null);
        Method get = cls("Effects").getMethod("getCount");
        cls("Effects").getMethod("setCount", int.class).invoke(effects, 0);
        equal(invoke("tracked", 3), 3);
        equal(get.invoke(effects), 1);
        Object pure = box("Tracked", -5);
        equal(unbox(pure), -5);
        equal(get.invoke(effects), 1);
        try { invoke("tracked", -1); throw new AssertionError("missing constructor exception"); }
        catch (InvocationTargetException expected) { yes(expected.getCause() instanceof IllegalArgumentException); equal(expected.getCause().getMessage(), "negative"); }
        equal(get.invoke(effects), 2);
        System.out.println("OK " + assertions);
    }
}

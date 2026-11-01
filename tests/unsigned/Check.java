import java.lang.reflect.*;
import java.math.BigInteger;
import java.util.*;

public final class Check {
    static int assertions;
    static Class<?> top;
    static final Map<String, Method> methods = new HashMap<>();
    static void equal(Object actual, Object expected) {
        assertions++;
        if (!Objects.equals(actual, expected)) throw new AssertionError("assertion " + assertions + ": " + actual + " != " + expected);
    }
    static Method method(String name) throws Exception {
        Method existing = methods.get(name);
        if (existing != null) return existing;
        for (Method method : top.getDeclaredMethods()) {
            if (Modifier.isPublic(method.getModifiers()) && !method.isSynthetic() && (method.getName().equals(name) || method.getName().startsWith(name + "-"))) {
                methods.put(name, method);
                return method;
            }
        }
        throw new NoSuchMethodException(name);
    }
    static Object call(String name, Object... args) throws Exception { return method(name).invoke(null, args); }
    static Object box(String owner, Object raw) throws Exception {
        Class<?> cls = Class.forName("kotlin." + owner);
        return cls.getMethod("box-impl", owner.equals("UInt") ? int.class : long.class).invoke(null, raw);
    }
    static Object unbox(Object value) throws Exception { return value.getClass().getMethod("unbox-impl").invoke(value); }
    static void fails(String name, Class<?> error, Object... args) throws Exception {
        try { call(name, args); throw new AssertionError("missing exception " + name); }
        catch (InvocationTargetException failure) { equal(failure.getCause().getClass(), error); }
    }
    static Object instance(Object receiver, String prefix, Object... args) throws Exception {
        for (Method method : receiver.getClass().getDeclaredMethods()) {
            if (Modifier.isPublic(method.getModifiers()) && !Modifier.isStatic(method.getModifiers()) && method.getParameterCount() == args.length && (method.getName().equals(prefix) || method.getName().startsWith(prefix + "-"))) return method.invoke(receiver, args);
        }
        throw new NoSuchMethodException(prefix);
    }
    public static void main(String[] args) throws Exception {
        top = Class.forName("unsignedcases.ScalarsKt");
        equal(call("uintZero"), 0);
        equal(call("separatedSuffix"), 1);
        equal(call("inferredAdd", -1, 1), 0);
        equal(call("extensionCall", -1), 0);
        equal(call("inferredConversion", -1L), -1);
        equal(call("uintMax"), -1);
        equal(call("ulongMax"), -1L);
        equal(call("inferredWide"), 4294967296L);
        equal(call("contextualWide"), 1L);
        equal(call("binaryLiteral"), 240);
        int[] ints = {0, 1, 2, -1, Integer.MIN_VALUE, Integer.MAX_VALUE};
        long[] longs = {0L, 1L, 2L, -1L, Long.MIN_VALUE, Long.MAX_VALUE};
        for (int a : ints) {
            equal(call("intToUInt", a), a);
            equal(call("uintToLong", a), Integer.toUnsignedLong(a));
            equal(call("uintToULong", a), Integer.toUnsignedLong(a));
            equal(call("uintText", a), Integer.toUnsignedString(a));
            equal(call("uintInvert", a), ~a);
            equal(call("addLiteral", a), a + 1);
            equal(call("uintGeneric", a), a);
            equal(unbox(call("uintBox", a)), a);
            equal(unbox(((List<?>)call("uintList", a)).get(0)), a);
            equal(call("smallByte", a), (byte)a);
            equal(call("byteToUInt", (byte)a), Byte.toUnsignedInt((byte)a));
            equal(call("uintNullable", box("UInt", a)), box("UInt", a));
            equal(call("uintElvis", box("UInt", a)), a);
            equal(call("nullableEquals", box("UInt", a)), a == 0);
            for (int b : ints) {
                equal(call("uintAdd", a, b), a + b);
                equal(call("uintSubtract", a, b), a - b);
                equal(call("uintMultiply", a, b), a * b);
                equal(Integer.signum((int)call("uintCompare", a, b)), Integer.signum(Integer.compareUnsigned(a, b)));
                equal(call("uintLess", a, b), Integer.compareUnsigned(a, b) < 0);
                equal(call("uintAnd", a, b), a & b);
                equal(call("uintOr", a, b), a | b);
                equal(call("uintXor", a, b), a ^ b);
                equal(call("uintShift", a, b), a >>> b);
                equal(call("uintShiftLeft", a, b), a << b);
                if (b != 0) {
                    equal(call("uintDivide", a, b), Integer.divideUnsigned(a, b));
                    equal(call("uintRemainder", a, b), Integer.remainderUnsigned(a, b));
                }
            }
        }
        for (long a : longs) {
            equal(call("longToULong", a), a);
            equal(call("ulongToInt", a), (int)a);
            equal(call("ulongToUInt", a), (int)a);
            equal(call("ulongText", a), Long.toUnsignedString(a));
            equal(call("ulongInvert", a), ~a);
            equal(call("ulongGeneric", a), a);
            equal(unbox(call("ulongBox", a)), a);
            equal(call("smallShort", a), (short)a);
            equal(call("shortToULong", (short)a), (long)Short.toUnsignedInt((short)a));
            equal(call("literalEqualsNullable", box("ULong", a)), a == 0);
            for (long b : longs) {
                BigInteger ua = new BigInteger(Long.toUnsignedString(a));
                BigInteger ub = new BigInteger(Long.toUnsignedString(b));
                equal(call("ulongAdd", a, b), ua.add(ub).longValue());
                equal(call("ulongSubtract", a, b), ua.subtract(ub).longValue());
                equal(call("ulongMultiply", a, b), ua.multiply(ub).longValue());
                equal(Integer.signum((int)call("ulongCompare", a, b)), Integer.signum(ua.compareTo(ub)));
                equal(call("ulongAtLeast", a, b), ua.compareTo(ub) >= 0);
                equal(call("ulongXor", a, b), a ^ b);
                equal(call("ulongShift", a, (int)b), a >>> (int)b);
                equal(call("mixedAdd", (int)a, b), Integer.toUnsignedLong((int)a) + b);
                if (b != 0) {
                    equal(call("ulongDivide", a, b), ua.divide(ub).longValue());
                    equal(call("ulongRemainder", a, b), ua.remainder(ub).longValue());
                }
            }
        }
        Object captured = call("lambdaCapture", -1);
        Method invoke = captured.getClass().getMethod("invoke");
        invoke.setAccessible(true);
        equal(unbox(invoke.invoke(captured)), 0);
        equal(call("uintIdentityNull", (Object)null), true);
        equal(call("ulongIdentityPresent", (Object)null), false);
        equal(call("uintIdentityNull", box("UInt", 0)), false);
        equal(call("ulongIdentityPresent", box("ULong", 0L)), true);
        equal(call("uintNullable", (Object)null), null);
        equal(call("uintElvis", (Object)null), 7);
        equal(call("nullableEquals", (Object)null), false);
        equal(call("literalEqualsNullable", (Object)null), false);
        fails("uintDivide", ArithmeticException.class, -1, 0);
        fails("uintRemainder", ArithmeticException.class, -1, 0);
        fails("ulongDivide", ArithmeticException.class, -1L, 0L);
        fails("ulongRemainder", ArithmeticException.class, -1L, 0L);
        StringBuilder log = new StringBuilder();
        equal(call("ordered", log), 0);
        equal(log.toString(), "LR");
        log.setLength(0);
        fails("orderedFailure", IllegalStateException.class, log);
        equal(log.toString(), "L");
        Class<?> marker = Class.forName("kotlin.jvm.internal.DefaultConstructorMarker");
        Class<?> pairClass = Class.forName("unsignedcases.ScalarPair");
        Object pair = pairClass.getConstructor(int.class, long.class, marker).newInstance(-1, -1L, null);
        equal(pair.toString(), "ScalarPair(first=4294967295, second=18446744073709551615)");
        equal(instance(pair, "getFirst"), -1);
        equal(instance(pair, "getSecond"), -1L);
        equal(instance(pair, "component1"), -1);
        equal(instance(pair, "component2"), -1L);
        equal(instance(pair, "copy", -1, -1L), pair);
        equal(pair.hashCode(), 31 * Integer.hashCode(-1) + Long.hashCode(-1L));
        equal(pairClass.getDeclaredField("first").getType(), int.class);
        equal(pairClass.getDeclaredField("second").getType(), long.class);
        equal(method("uintNullable").getReturnType(), Class.forName("kotlin.UInt"));
        equal(method("uintGeneric").getReturnType(), int.class);
        System.out.println("OK " + assertions);
    }
}

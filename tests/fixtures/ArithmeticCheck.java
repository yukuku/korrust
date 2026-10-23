import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.function.IntBinaryOperator;
import korrust.probes.ArithmeticKt;

public final class ArithmeticCheck {
    private static final int[] VALUES = {Integer.MIN_VALUE, Integer.MIN_VALUE + 1, -100, -33, -32, -1, 0, 1, 2, 31, 32, 33, 100, Integer.MAX_VALUE - 1, Integer.MAX_VALUE};
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception e) { throw new ExceptionInInitializerError(e); }
    }

    private static void record(Object actual, Object expected) {
        if (!actual.equals(expected)) {
            throw new AssertionError(actual + " != " + expected);
        }
        DIGEST.update((actual.toString() + "\n").getBytes(StandardCharsets.UTF_8));
        checks++;
    }

    private static void binary(String name, IntBinaryOperator oracle) throws Exception {
        Method method = ArithmeticKt.class.getMethod(name, int.class, int.class);
        for (int a : VALUES) {
            for (int b : VALUES) {
                Object expected;
                try { expected = oracle.applyAsInt(a, b); }
                catch (ArithmeticException e) { expected = e.getClass().getName(); }
                Object actual;
                try { actual = method.invoke(null, a, b); }
                catch (InvocationTargetException e) { actual = e.getCause().getClass().getName(); }
                record(actual, expected);
            }
        }
    }

    private static void exception(Class<? extends Throwable> expected, Runnable action) {
        try {
            action.run();
            throw new AssertionError("Expected " + expected.getName());
        } catch (Throwable actual) {
            record(actual.getClass().getName(), expected.getName());
        }
    }

    public static void main(String[] args) throws Exception {
        binary("add", (a, b) -> a + b);
        binary("subtract", (a, b) -> a - b);
        binary("multiply", (a, b) -> a * b);
        binary("divide", (a, b) -> a / b);
        binary("remainder", (a, b) -> a % b);
        binary("left", (a, b) -> a << b);
        binary("right", (a, b) -> a >> b);
        binary("unsignedRight", (a, b) -> a >>> b);
        binary("bitAnd", (a, b) -> a & b);
        binary("bitOr", (a, b) -> a | b);
        binary("bitXor", (a, b) -> a ^ b);
        binary("compare", Integer::compare);
        binary("midpoint", (a, b) -> (a + b) >>> 1);
        binary("precedence", (a, b) -> ((a + 1) << b) ^ 3);
        record(ArithmeticKt.minimum(), Integer.MIN_VALUE);
        record(ArithmeticKt.maximum(), Integer.MAX_VALUE);
        for (int a : VALUES) {
            record(ArithmeticKt.negate(a), -a);
            record(ArithmeticKt.scope(a), a > 0 ? 4 : 7);
            record(ArithmeticKt.bothReturn(a), a > 0 ? 11 : 13);
            for (int b : VALUES) {
                record(ArithmeticKt.less(a, b), a < b);
                record(ArithmeticKt.greater(a, b), a > b);
                record(ArithmeticKt.lessEqual(a, b), a <= b);
                record(ArithmeticKt.greaterEqual(a, b), a >= b);
                record(ArithmeticKt.equal(a, b), a == b);
                record(ArithmeticKt.notEqual(a, b), a != b);
                record(ArithmeticKt.conditions(a, b), a > b && a != 0 || a == b);
            }
        }
        for (int n = -1; n <= 100; n++) {
            record(ArithmeticKt.sum(n), n < 0 ? 0 : n * (n - 1) / 2);
        }
        int[] fib = {0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144};
        for (int n = 0; n < fib.length; n++) { record(ArithmeticKt.recursive(n), fib[n]); }
        int[] a = {Integer.MIN_VALUE, 0, Integer.MAX_VALUE};
        record(ArithmeticKt.echo(a) == a, true);
        record(ArithmeticKt.size(a), 3);
        for (int i = 0; i < a.length; i++) { record(ArithmeticKt.read(a, i), a[i]); }
        record(ArithmeticKt.guarded(false, new int[0]), false);
        record(ArithmeticKt.orGuarded(true, new int[0]), true);
        record(ArithmeticKt.guarded(true, new int[]{1}), true);
        record(ArithmeticKt.orGuarded(false, new int[]{-1}), false);
        record(ArithmeticKt.invert(true), false);
        record(ArithmeticKt.invert(false), true);
        exception(ArrayIndexOutOfBoundsException.class, () -> ArithmeticKt.guarded(true, new int[0]));
        exception(ArrayIndexOutOfBoundsException.class, () -> ArithmeticKt.orGuarded(false, new int[0]));
        exception(ArrayIndexOutOfBoundsException.class, () -> ArithmeticKt.read(a, -1));
        exception(ArrayIndexOutOfBoundsException.class, () -> ArithmeticKt.read(a, a.length));
        exception(NullPointerException.class, () -> ArithmeticKt.size(null));
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

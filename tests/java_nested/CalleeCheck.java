package calleecases;
import callee.Arena;
public final class CalleeCheck {
    static int assertions;
    static void equal(Object actual, Object expected) { assertions++; if (!actual.equals(expected)) throw new AssertionError(actual + " != " + expected); }
    public static void main(String[] args) {
        Arena.trace = "";
        equal(CalleesKt.constructorOnly().getClass(), Arena.CtorOnly.class);
        equal(Arena.trace, "C");
        Arena.trace = "";
        equal(CalleesKt.methodOnly(), "method");
        equal(Arena.trace, "m");
        StringBuilder log = new StringBuilder();
        Arena.trace = "";
        equal(CalleesKt.constructorSpecific(log).value, "x");
        equal(log.toString(), "A");
        equal(Arena.trace, "S");
        log.setLength(0);
        Arena.trace = "";
        equal(CalleesKt.methodSpecific(log), "string-method");
        equal(log.toString(), "A");
        equal(Arena.trace, "M");
        Arena.trace = "";
        equal(CalleesKt.privateFactory(), "private");
        equal(Arena.trace, "P");
        Arena.trace = "";
        equal(CalleesKt.innerFactory(), "inner");
        equal(Arena.trace, "I");
        Arena.trace = "";
        equal(CalleesKt.hiddenFactory(), "hidden");
        equal(Arena.trace, "H");
        Arena.trace = "";
        equal(CalleesKt.abstractFactory(), "abstract");
        equal(Arena.trace, "B");
        equal(CalleesKt.aliasMethod(), "method");
        equal(CalleesKt.fullyQualified(), "method");
        Arena.trace = "";
        equal(CalleesKt.methodOnConstructorName(), "arity");
        equal(Arena.trace, "c");
        Arena.trace = "";
        equal(CalleesKt.constructorOnMethodName().value, 5);
        equal(Arena.trace, "n");
        System.out.println("OK " + assertions);
    }
}

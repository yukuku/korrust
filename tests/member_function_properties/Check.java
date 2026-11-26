import memberfunctions.*;

public final class Check {
    public static void main(String[] args) {
        Captured captured = new Captured(3);
        Request request = captured.request(5);
        if (request.execute() != 8) throw new AssertionError("capture");
        Request nested = captured.factory().invoke(5);
        if (nested.execute() != 8) throw new AssertionError("nested capture");
        captured.replace(4);
        if (request.execute() != 20 || nested.execute() != 20) throw new AssertionError("mutable dispatch");
        Target target = new Target();
        if (ValuesKt.order(target) != 15 || target.getTrace() != 1234) throw new AssertionError("order " + target.getTrace());
        if (ValuesKt.nullable(null) != null || ValuesKt.nullable(new Target()) != 17) throw new AssertionError("safe receiver");
        if (new Both().action(1) != 41 || new Both().own() != 42) throw new AssertionError("method precedence");
        if (target.action("abc") != 103 || target.own() != 15) throw new AssertionError("method overload");
        if (ValuesKt.dispatch(new Multiplied(7)) != 21) throw new AssertionError("interface dispatch");
        if (!ValuesKt.generic(new Generic<String>(value -> value + "!")).equals("value!")) throw new AssertionError("generic");
        if (ValuesKt.extension(new ExtensionHolder((value, count) -> value.length() + count)) != 5) throw new AssertionError("receiver function");
        if (ValuesKt.singleton() != 54 || memberfunctionlibrary.Callbacks.INSTANCE.getReads() != 12) throw new AssertionError("static getter");
        System.out.println("OK");
    }
}

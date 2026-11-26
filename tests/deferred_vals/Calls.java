package deferredvalscheck;
import deferredvals.ValuesKt;
public class Calls {
    public static void main(String[] args) {
        if (ValuesKt.choose(true) != 3 || ValuesKt.choose(false) != 4) throw new AssertionError("if");
        if (ValuesKt.continuing(true) != 5 || ValuesKt.continuing(false) != 8) throw new AssertionError("terminal path");
        if (ValuesKt.expression(true) != 6 || ValuesKt.expression(false) != 7) throw new AssertionError("if expression");
        if (ValuesKt.selected(0) != 10 || ValuesKt.selected(1) != 20 || ValuesKt.selected(9) != 30) throw new AssertionError("when");
        if (ValuesKt.selectedExpression(true) != 12 || ValuesKt.selectedExpression(false) != 24) throw new AssertionError("when expression");
        if (ValuesKt.nested(0) != 0 || ValuesKt.nested(4) != 24) throw new AssertionError("loop local vals");
        if (ValuesKt.shadow(true) != 12 || ValuesKt.shadow(false) != 9) throw new AssertionError("shadow");
        if (!ValuesKt.reference(true).equals("left") || !ValuesKt.reference(false).equals("right")) throw new AssertionError("reference");
        if (ValuesKt.wide(true) != 4294967298L || ValuesKt.wide(false) != 4294967299L) throw new AssertionError("wide");
        if (ValuesKt.terminal(true) != 7 || ValuesKt.terminal(false) != 8) throw new AssertionError("abrupt rhs");
        if (ValuesKt.capture(true).invoke().intValue() != 9 || ValuesKt.capture(false).invoke().intValue() != 10) throw new AssertionError("initialized capture");
        System.out.println("deferred-vals-ok");
    }
}

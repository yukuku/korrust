import operators.Events;
import operators.Measure;
import operators.Shifted;
import operators.OperatorsKt;
import imported.ImportedKt;

public class Check {
    static int assertions;
    static void check(boolean ok, String message) { assertions++; if (!ok) throw new AssertionError(message); }
    static void trace(String expected) { check(Events.INSTANCE.getTrace().equals(expected), "trace " + Events.INSTANCE.getTrace() + " != " + expected); }
    public static void main(String[] args) {
        Events.INSTANCE.reset();
        check(OperatorsKt.sourceUnary() == 111, "member unary result"); trace("L-+!");
        Events.INSTANCE.reset();
        check(OperatorsKt.extensionUnary() == -4, "extension unary"); trace("mp");
        Events.INSTANCE.reset();
        check(OperatorsKt.extensionNot().equals("custom"), "not return is unrestricted"); trace("n");
        Events.INSTANCE.reset();
        check(OperatorsKt.precedence() == 199, "unary/property precedence"); trace("!");
        for (int a : new int[]{-2, 0, 2}) for (int b : new int[]{-2, 0, 2}) for (int mode = 0; mode < 4; mode++) {
            boolean expected = mode == 0 ? a < b : mode == 1 ? a <= b : mode == 2 ? a > b : a >= b;
            Events.INSTANCE.reset();
            check(OperatorsKt.compare(a, b, mode) == expected, "source compare"); trace("LRc");
            check(ImportedKt.compareMember(a, b, mode) == expected, "compiled compare");
        }
        Events.INSTANCE.reset();
        check(OperatorsKt.extensionCompare(2, 3), "source extension comparison"); trace("e");
        Events.INSTANCE.reset();
        check(OperatorsKt.dispatch(new Shifted(4)) == 24, "virtual unary"); trace("d");
        Events.INSTANCE.reset();
        check(OperatorsKt.dispatchCompare(new Shifted(4)), "virtual compare"); trace("v");
        Events.INSTANCE.reset();
        try { OperatorsKt.unaryFailure(); throw new AssertionError("missing exception"); }
        catch (IllegalStateException expected) { check(expected.getMessage().equals("operand"), "unary exception"); } trace("F");
        Events.INSTANCE.reset();
        try { OperatorsKt.compareFailure(); throw new AssertionError("missing exception"); }
        catch (IllegalStateException expected) { check(expected.getMessage().equals("operand"), "compare exception"); } trace("LF");
        check(ImportedKt.members() == 2, "compiled unary member");
        check(ImportedKt.notMember() == 507L, "compiled non-Boolean not");
        check(ImportedKt.extensions() == -4, "compiled unary extension");
        check(ImportedKt.notExtension().equals("external"), "compiled not extension");
        check(ImportedKt.compareExtension(2, 3), "compiled compare extension");
        unusual.Trace.INSTANCE.reset();
        check(ImportedKt.compiledOrder(), "compiled extension operand order");
        check(unusual.Trace.INSTANCE.getTrace().equals("LRC"), "compiled extension receiver evaluated once before argument");
        unusual.Trace.INSTANCE.reset();
        check(ImportedKt.compiledUnaryOrder().equals("external"), "compiled unary operand");
        check(unusual.Trace.INSTANCE.getTrace().equals("LN"), "compiled unary receiver evaluated once");
        check(ImportedKt.interfaceCall(new unusual.Ranked(-1), new unusual.Ranked(1)), "compiled interface compare");
        check(ImportedKt.compiledInherited() == 34, "compiled inherited unary modifier");
        check(ImportedKt.compiledInheritedCompare(), "compiled inherited compare modifier");
        check(OperatorsKt.sourceInterface(new operators.Switch()) == 7, "source interface inherited operator");
        check(OperatorsKt.sourceImplementation() == 7, "inferred implementation inherited operator");
        check(OperatorsKt.grandparentInherited() == 3, "grandparent operator modifier");
        unusual.Down covariant = new unusual.Down();
        check(ImportedKt.compiledCovariant(covariant) == covariant, "compiled covariant unary modifier");
        unusual.Up base = covariant;
        check(base.unaryMinus() == covariant, "compiled covariant unary virtual bridge");
        check(OperatorsKt.directDispatch() == 24, "inherited unary operator modifier");
        check(OperatorsKt.directCompare(), "inherited compare operator modifier");
        check(new Measure(2).unaryPlus().getValue() == 12, "Java unary ABI");
        check(new Measure(2).compareTo(new Measure(3)) == Integer.MIN_VALUE, "Java compare ABI");
        System.out.println("OK " + assertions);
    }
}

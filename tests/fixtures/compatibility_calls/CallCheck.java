public final class CallCheck {
    public static void main(String[] args) {
        String actual = switch (args[0]) {
            case "genericConstraints" -> compatibility.calls.GenericConstraintsKt.genericConstraints();
            case "compiledDefaults" -> compatibility.calls.CompiledDefaultsKt.compiledDefaults();
            case "constructorDefaults" -> compatibility.calls.ConstructorDefaultsKt.constructorDefaults();
            case "maskBoundary" -> compatibility.calls.MaskBoundaryKt.maskBoundary();
            case "ordinaryVarargs" -> compatibility.calls.OrdinaryVarargsKt.ordinaryVarargs();
            case "sourceVarargs" -> compatibility.calls.SourceVarargsKt.sourceVarargs();
            case "spreadArguments" -> compatibility.calls.SpreadArgumentsKt.spreadArguments();
            case "spreadExceptions" -> compatibility.calls.SpreadExceptionsKt.spreadExceptions();
            case "builtinShadow" -> compatibility.calls.BuiltinShadowKt.builtinShadow();
            case "arrayReturnBody" -> compatibility.calls.ArrayReturnBodyKt.arrayReturnBody();
            case "platformJoins" -> PlatformJoinCheck.check();
            default -> throw new AssertionError(args[0]);
        };
        String expected = switch (args[0]) {
            case "genericConstraints" -> "null:9:ab:null:3:ok";
            case "compiledDefaults" -> "129:LAB|823:FBC|829:LFB|123:ABC:none:ok";
            case "constructorDefaults" -> "1:9:LA|1347:RZD";
            case "maskBoundary" -> "96:333:372";
            case "ordinaryVarargs" -> "0:4:5:13:9:104:2:AB:java.lang.Integer:java.lang.String";
            case "sourceVarargs" -> "0:4:6";
            case "spreadArguments" -> "2:2:10:a:a:SABCT";
            case "spreadExceptions" -> "ABX";
            case "builtinShadow" -> "103";
            case "arrayReturnBody" -> "12:2:SQ";
            case "platformJoins" -> "known";
            default -> throw new AssertionError(args[0]);
        };
        if (!actual.equals(expected)) throw new AssertionError(args[0] + ": " + actual + " != " + expected);
        System.out.println(actual);
    }
}

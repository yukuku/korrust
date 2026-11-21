package trailing;
import java.util.Arrays;
import java.util.Objects;
public final class Calls {
    private static void eq(Object expected, Object actual) {
        if (!Objects.equals(expected, actual)) throw new AssertionError("expected=" + expected + " actual=" + actual);
    }
    public static void main(String[] args) {
        eq("source2!", ModelsKt.sourceDefault());
        eq("source4!", ModelsKt.sourceNamed());
        eq("7", ModelsKt.sourceGeneric());
        eq("dep1:5", ModelsKt.compiledExtension());
        eq("dep6:5", ModelsKt.compiledNamed());
        eq("8", ModelsKt.compiledNullable());
        eq("member9", ModelsKt.compiledMember());
        eq("string!", ModelsKt.compiledOverload());
        eq("4!", ModelsKt.selectedOverload());
        eq("source2label", ModelsKt.explicitLabel());
        eq("source2label", ModelsKt.implicitLabel());
        eq("early", ModelsKt.inlineReturn());
        eq("01 80 ff", ModelsKt.joined(Arrays.asList((byte) 1, (byte) 128, (byte) 255)));
        eq("", ModelsKt.joined(Arrays.asList()));
        eq("AB:A4B", ModelsKt.ordered());
        eq("array", ModelsKt.arrayInitializer());
        eq("constructor", ModelsKt.constructed());
        eq("2function", ModelsKt.functionValue());
        eq("source2captured", ModelsKt.captured());
        System.out.println("OK");
    }
}

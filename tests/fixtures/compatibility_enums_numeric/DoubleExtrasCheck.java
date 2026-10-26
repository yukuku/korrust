import compatibility.enumsnumeric.*;

public class DoubleExtrasCheck {
    public static void main(String[] args) throws Exception {
        if (Class.forName("compatibility.enumsnumeric.DoubleExtrasKt").getField("constant").getDouble(null) != 1.25) throw new AssertionError("constant field");
        if (DoubleExtrasKt.leading() != 0.5 || !Double.isNaN(DoubleExtrasKt.sourceNan()) || !Double.isNaN(DoubleExtrasKt.explicitJavaNan()) || DoubleExtrasKt.sourceInfinity()!=Double.POSITIVE_INFINITY) throw new AssertionError("literals/companion/JVM fields");
        if (DoubleExtrasKt.defaultProbe()!=4.0 || new DefaultBox().getValue()!=2.5) throw new AssertionError("defaults");
        if (DoubleExtrasKt.capture(1.0).invoke()!=1.25) throw new AssertionError("capture");
        kotlin.jvm.functions.Function0<Double> next=DoubleExtrasKt.counter(1.0);
        if (next.invoke()!=1.5 || next.invoke()!=2.0) throw new AssertionError("mutable Double capture");
        if (!DoubleExtrasKt.stringValue(-0.0).equals("-0.0")) throw new AssertionError("string");
        System.out.println("double-extras-ok");
    }
}

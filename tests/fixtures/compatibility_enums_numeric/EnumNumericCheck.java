import compatibility.enumsnumeric.*;

public class EnumNumericCheck {
    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void bits(double actual, double expected, String message) {
        require(Double.doubleToLongBits(actual) == Double.doubleToLongBits(expected), message + ": " + actual + " != " + expected);
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    public static void main(String[] args) throws Exception {
        Signal red = Signal.RED;
        require(EnumTrace.INSTANCE.getText().equals("15269"), "enum argument/init/companion order: " + EnumTrace.INSTANCE.getText());
        require(red.name().equals("RED") && red.ordinal() == 0 && red.getCode() == 1, "RED user and implicit args");
        require(Signal.GREEN.name().equals("GREEN") && Signal.GREEN.ordinal() == 1 && Signal.GREEN.getCode() == 2, "GREEN user and implicit args");
        require(red.getDoubled() == 2 && red.caption().equals("red:1"), "enum members");
        require(Signal.Companion.fromCode(1) == red && Signal.Companion.fromCode(2) == Signal.GREEN, "enum companion");
        require(Signal.Companion.getInitialized() == 9, "companion initialization");
        Signal[] values = Signal.values();
        values[0] = Signal.GREEN;
        require(Signal.values()[0] == red, "values must copy");
        require(Signal.getEntries() == Signal.getEntries() && Signal.getEntries().get(0) == red && Signal.getEntries().get(1) == Signal.GREEN, "entries stable identity/order");
        require(Empty.values().length == 0 && Empty.getEntries().isEmpty(), "empty enum");
        try { Signal.valueOf("missing"); throw new AssertionError("valueOf accepted missing"); } catch (IllegalArgumentException expected) { }
        Class<?> gotoTab = Class.forName("yuku.alkitab.base.compose.goto.GotoTab");
        Object[] tabs = (Object[]) gotoTab.getMethod("values").invoke(null);
        require(tabs.length == 3, "all GotoTab entries");
        for (int index = 0; index < tabs.length; index++) require(((Enum<?>) tabs[index]).ordinal() == index && ((Integer) gotoTab.getMethod("getOrdinal1Based").invoke(tabs[index])) == index + 1, "GotoTab " + index);
        require(((java.util.List<?>) gotoTab.getMethod("getEntries").invoke(null)).get(2) == tabs[2], "GotoTab entries");
        double[] boundaries = {0.0, -0.0, 1.5, -1.5, Double.MIN_VALUE, Double.MAX_VALUE, Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, 2147483647.9, -2147483649.0, 9.223372036854776E18};
        for (double first : boundaries) {
            bits(DoubleModel.scalar(first), first, "scalar");
            require(DoubleModel.boxed(first).equals(first), "boxed return");
            require(DoubleModel.toInt(first) == (int) first, "toInt " + first);
            require(DoubleModel.toLong(first) == (long) first, "toLong " + first);
            require(Float.floatToIntBits(DoubleModel.toFloat(first)) == Float.floatToIntBits((float) first), "toFloat " + first);
            for (double second : boundaries) {
                require(DoubleModel.primitiveEqual(first, second) == (first == second), "primitive equality " + first + "/" + second);
                require(DoubleModel.boxedEqual(first, second) == (first == second), "nullable statically Double equality " + first + "/" + second);
                require(DoubleModel.genericEqual(first, second) == Double.valueOf(first).equals(second), "generic equality " + first + "/" + second);
                require(DoubleModel.less(first, second) == (first < second), "less");
                require(DoubleModel.lessEqual(first, second) == (first <= second), "lessEqual");
                require(DoubleModel.greater(first, second) == (first > second), "greater");
                require(DoubleModel.greaterEqual(first, second) == (first >= second), "greaterEqual");
                bits(DoubleModel.arithmetic(first, second), ((first + second) * second - first) / second % 3.0, "arithmetic");
            }
        }
        require(DoubleModel.boxed(null) == null && DoubleModel.boxedEqual(null, null), "nullable null boxing");
        require(!DoubleModel.boxedEqual(null, 0.0) && !DoubleModel.boxedEqual(0.0, null), "nullable equality");
        bits(DoubleModel.minLiteral(), Double.MIN_VALUE, "minimum subnormal literal");
        bits(DoubleModel.maxLiteral(), Double.MAX_VALUE, "maximum literal");
        bits(DoubleModel.overflowLiteral(), Double.POSITIVE_INFINITY, "overflow literal");
        bits(DoubleModel.underflowLiteral(), 0.0, "underflow literal");
        bits(DoubleModel.genericRoundTrip(-0.0), -0.0, "generic Double boxing and unboxing");
        bits(DoubleModel.zeroLiteral(), -0.0, "negative zero literal");
        bits(DoubleModel.fromInt(Integer.MIN_VALUE), (double) Integer.MIN_VALUE, "Int conversion");
        bits(DoubleModel.fromLong(Long.MAX_VALUE), (double) Long.MAX_VALUE, "Long conversion");
        bits(DoubleModel.fromFloat(Float.NaN), Double.NaN, "Float conversion");
        bits(DoubleModel.mixed(0.5, 2, 4L, 1.25f), 7.75, "mixed values");
        bits(DoubleModel.slots(3, 1.25, 9L, 2.5, true), 15.75, "two-slot true");
        bits(DoubleModel.slots(3, 1.25, 9L, 2.5, false), 15.75, "two-slot false");
        bits(DoubleModel.nullable(null, -0.0), -0.0, "Elvis null");
        bits(DoubleModel.nullable(2.5, 8.0), 2.5, "Elvis present");
        try { DoubleModel.asserted(null); throw new AssertionError("null assertion did not fail"); } catch (NullPointerException expected) { }
        bits(DoubleModel.javaCalls(-9.0, 4.0), 9.0, "real java.lang.Math calls");
        java.util.List rawNumbers = java.util.Collections.singletonList(3.25f);
        bits(DoubleModel.listAssert(rawNumbers), 3.25, "immediate nullable generic Double assertion converts Number");
        bits(DoubleModel.listValue(rawNumbers), 3.25, "non-null generic Double result converts Number");
        try { DoubleModel.listNullable(rawNumbers); throw new AssertionError("materialized Double? omitted concrete cast"); } catch (ClassCastException expected) { }
        try { DoubleModel.listLocalAssert(rawNumbers); throw new AssertionError("local Double? omitted concrete cast"); } catch (ClassCastException expected) { }
        try { DoubleModel.listElvis(rawNumbers); throw new AssertionError("Elvis Double? omitted concrete cast"); } catch (ClassCastException expected) { }
        DoubleBox box = new DoubleBox(1.25, null);
        bits(box.add(2.5), 3.75, "Double field read/write");
        require(box.getOptional() == null, "boxed Double field");
        DoubleRecord nan = new DoubleRecord(Double.NaN, null);
        require(nan.equals(new DoubleRecord(Double.NaN, null)), "data class Double.NaN equality");
        require(nan.hashCode() == 31 * Double.hashCode(Double.NaN), "data class Double hash");
        require(nan.toString().equals("DoubleRecord(value=NaN, optional=null)"), "data class Double string");
        require(!new DoubleRecord(0.0, null).equals(new DoubleRecord(-0.0, null)), "data class zero sign equality");
        bits(nan.copy(4.0, 2.5).component1(), 4.0, "data class copy/component");
        System.out.println("java-enums-numeric-ok");
    }
}

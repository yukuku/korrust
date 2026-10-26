public final class NullableDoubleBoundaryCheck {
    private static int checks;

    private static void check(boolean condition, String message) {
        checks++;
        if (!condition) throw new AssertionError(message);
    }

    private static void checkValue(Double input, long expectedBits) {
        Double output = Boundary_nullable_doubleKt.f(input);
        check(output == input, "nullable Double identity");
        check(Double.doubleToRawLongBits(output) == expectedBits, "nullable Double raw bits");
    }

    public static void main(String[] args) {
        check(Boundary_nullable_doubleKt.f(null) == null, "null preservation");
        Double nan = Double.valueOf(Double.NaN);
        Double nanResult = Boundary_nullable_doubleKt.f(nan);
        check(nanResult == nan, "boxed NaN identity");
        check(Double.isNaN(nanResult), "NaN preservation");
        checkValue(Double.valueOf(Double.longBitsToDouble(0x7ff8000000000042L)), 0x7ff8000000000042L);
        checkValue(Double.valueOf(0.0), 0x0000000000000000L);
        checkValue(Double.valueOf(-0.0), 0x8000000000000000L);
        checkValue(Double.valueOf(Double.POSITIVE_INFINITY), 0x7ff0000000000000L);
        checkValue(Double.valueOf(Double.NEGATIVE_INFINITY), 0xfff0000000000000L);
        checkValue(Double.valueOf(Double.MAX_VALUE), 0x7fefffffffffffffL);
        checkValue(Double.valueOf(Double.MIN_VALUE), 0x0000000000000001L);
        checkValue(Double.valueOf(3.25), 0x400a000000000000L);
        checkValue(Double.valueOf(-7.5), 0xc01e000000000000L);
        System.out.println("{\"id\":\"nullable_double\",\"checks\":" + checks + "}");
    }
}

package flowuse;
public class Driver {
    static void check(boolean condition) { if (!condition) throw new AssertionError(); }
    public static void main(String[] args) {
        int[] integers = {8, Integer.MAX_VALUE};
        check(UseKt.integer(integers) == -3);
        check(integers[1] == Integer.MIN_VALUE);
        long[] longs = {Long.MAX_VALUE};
        check(UseKt.wide(longs) == Long.MAX_VALUE - 1);
        check(longs[0] == Long.MIN_VALUE);
        check(UseKt.summed(new double[] {1.25, -2.5, 10.0}) == 17.5);
        check(UseKt.summed(new double[0]) == 0.0);
        check(Double.isNaN(UseKt.summed(new double[] {Double.NaN})));
        check(UseKt.converted(5.5) == -6.0);
        check(UseKt.rounded(2.5) == 2.0 && UseKt.rounded(3.5) == 4.0);
        check(Double.doubleToRawLongBits(UseKt.rounded(-0.2)) == Long.MIN_VALUE);
        check(Double.isNaN(UseKt.rounded(Double.NaN)));
        check(UseKt.rounded(Double.POSITIVE_INFINITY) == Double.POSITIVE_INFINITY);
        check(UseKt.switched(0) == 11 && UseKt.switched(1) == 21 && UseKt.switched(2) == 31);
        check(UseKt.switched(-1) == 0 && UseKt.switched(Integer.MAX_VALUE) == 0);
        check(UseKt.sparseSwitched(-10000) == 4 && UseKt.sparseSwitched(0) == 6);
        check(UseKt.sparseSwitched(10000) == 10 && UseKt.sparseSwitched(1) == 14);
        check(UseKt.allocated(5.5) == 0.0);
        check(Double.isNaN(UseKt.allocated(Double.POSITIVE_INFINITY)));
        check(UseKt.compared(2.0) == 3 && UseKt.compared(-2.0) == -1);
        check(UseKt.compared(Double.NaN) == 1 && UseKt.compared(0.5) == 1);
        check(UseKt.nonlocal(1) == 90 && UseKt.nonlocal(0) == -1);
        check(UseKt.capturedDefault(7) == 12);
        System.out.println("OK");
    }
}

import declarations.boundary.ArrayFactoryOrderKt;
import declarations.support.Trace;
public final class ArrayFactoryCheck {
    public static void main(String[] args) {
        Trace.clear();
        if (ArrayFactoryOrderKt.negativeArray() != 1) throw new AssertionError("negative size");
        if (!Trace.events.equals("")) throw new AssertionError("negative-size factory evaluation: " + Trace.events);
        if (ArrayFactoryOrderKt.zeroArray() != 2) throw new AssertionError("zero size");
        if (!Trace.events.equals("9")) throw new AssertionError("zero-size factory evaluation: " + Trace.events);
        System.out.println("{\"checks\":4,\"events\":\"9\"}");
    }
}

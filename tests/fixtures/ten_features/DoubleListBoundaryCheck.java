import java.util.AbstractList;
import java.util.Arrays;
import java.util.Collections;

public final class DoubleListBoundaryCheck {
    private static int checks;
    private static void check(boolean value, String message) { checks++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        check(Boundary_unsupported_list_scalarKt.f(Collections.<Double>emptyList()) == 0, "empty Double list");
        check(Boundary_unsupported_list_scalarKt.f(Arrays.asList(1.5, Double.NaN, -0.0)) == 3, "Double list size");
        int[] calls = {0};
        AbstractList<Double> traced = new AbstractList<Double>() {
            public int size() { calls[0]++; return 4; }
            public Double get(int index) { throw new AssertionError("size must not read elements"); }
        };
        check(Boundary_unsupported_list_scalarKt.f(traced) == 4, "virtual list size result");
        check(calls[0] == 1, "virtual list size called once");
        try { Boundary_unsupported_list_scalarKt.f(null); throw new AssertionError("null list accepted"); }
        catch (NullPointerException expected) { checks++; }
        System.out.println("{\"id\":\"unsupported_list_scalar\",\"checks\":" + checks + "}");
    }
}

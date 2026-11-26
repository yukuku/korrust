public final class SafeUnitBoundaryCheck {
    public static void main(String[] args) {
        ten.trace.Trace.trace = "";
        if (Boundary_safe_unit_resultKt.f(null) != null || !ten.trace.Trace.trace.isEmpty()) throw new AssertionError();
        if (Boundary_safe_unit_resultKt.f(new ten.trace.Trace()) != kotlin.Unit.INSTANCE || !ten.trace.Trace.trace.equals("R")) throw new AssertionError();
        System.out.println("{\"id\":\"safe_unit_result\",\"checks\":4}");
    }
}

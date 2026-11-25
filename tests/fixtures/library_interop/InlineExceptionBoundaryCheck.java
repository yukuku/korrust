public final class InlineExceptionBoundaryCheck {
    public static void main(String[] args) {
        if (Boundary_inline_exception_handlerKt.boundary() != 3) throw new AssertionError("normal");
        if (Boundary_inline_exception_handlerKt.failure() != -2) throw new AssertionError("caught");
        if (Boundary_inline_exception_handlerKt.nonlocal() != 5) throw new AssertionError("nonlocal");
        System.out.println("{\"id\":\"inline_exception_handler\",\"checks\":3}");
    }
}

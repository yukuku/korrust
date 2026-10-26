public final class InlineAllocationBoundaryCheck {
    private static int checks;
    private static void check(boolean value, String message) { checks++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        check(new Object().hashCode() == 1, "controlled HotSpot identity hash mode is required");
        for (int index = 0; index < 64; index++) {
            check(Boundary_inline_allocationKt.boundary() == 4, "inline body must add lambda result 3 and allocated Object hash 1");
        }
        System.out.println("{\"id\":\"inline_allocation\",\"checks\":" + checks + "}");
    }
}

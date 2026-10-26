public final class NullableEqualityBoundaryCheck {
    private static int checks;
    private static void check(boolean value, String message) { checks++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        check(Boundary_nullable_structural_equalityKt.f(null, null), "null equals null");
        check(!Boundary_nullable_structural_equalityKt.f(null, "a"), "null differs from nonnull");
        check(!Boundary_nullable_structural_equalityKt.f("a", null), "nonnull differs from null");
        check(Boundary_nullable_structural_equalityKt.f(new String("same"), new String("same")), "structural equality across identities");
        check(!Boundary_nullable_structural_equalityKt.f("same", "SAME"), "case-sensitive inequality");
        check(Boundary_nullable_structural_equalityKt.f("\u0000\ud83d\ude00", new String(new char[]{0, '\ud83d', '\ude00'})), "UTF-16 equality");
        System.out.println("{\"id\":\"nullable_structural_equality\",\"checks\":" + checks + "}");
    }
}

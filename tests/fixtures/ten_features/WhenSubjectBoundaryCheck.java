public final class WhenSubjectBoundaryCheck {
    private static int checks;
    private static void check(boolean value, String message) { checks++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        check(Boundary_when_subject_declarationKt.f("a") == 1, "matching subject");
        check(Boundary_when_subject_declarationKt.f(new String("a")) == 1, "structural String subject");
        check(Boundary_when_subject_declarationKt.f("") == 2, "empty subject fallback");
        check(Boundary_when_subject_declarationKt.f("A") == 2, "case-sensitive fallback");
        check(Boundary_when_subject_declarationKt.f("\u0000a") == 2, "UTF-16 subject fallback");
        try { Boundary_when_subject_declarationKt.f(null); throw new AssertionError("null subject accepted"); }
        catch (NullPointerException expected) { checks++; }
        System.out.println("{\"id\":\"when_subject_declaration\",\"checks\":" + checks + "}");
    }
}

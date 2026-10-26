import java.util.List;

public final class ListSpreadBoundaryCheck {
    private static int checks;
    private static void check(boolean value, String message) { checks++; if (!value) throw new AssertionError(message); }
    public static void main(String[] args) {
        check(Boundary_list_factory_spreadKt.f(new String[0]).isEmpty(), "empty spread");
        String[] input = {"a", "b"};
        List<String> result = Boundary_list_factory_spreadKt.f(input);
        check(result.size() == 2 && result.get(0).equals("a") && result.get(1).equals("b"), "spread contents/order");
        input[0] = "source";
        check(result.get(0).equals("a"), "spread copies input");
        result.set(1, "list");
        check(input[1].equals("b") && result.get(1).equals("list"), "copied list backing storage");
        check(Boundary_list_factory_spreadKt.f(new String[]{"single"}).get(0).equals("single"), "single spread");
        try { Boundary_list_factory_spreadKt.f(null); throw new AssertionError("null spread accepted"); }
        catch (NullPointerException expected) { checks++; }
        System.out.println("{\"id\":\"list_factory_spread\",\"checks\":" + checks + "}");
    }
}

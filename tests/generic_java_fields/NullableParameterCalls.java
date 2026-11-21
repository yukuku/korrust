package genericfields;
public final class NullableParameterCalls {
    public static void main(String[] args) {
        Cell<String> cell = new Cell<>("value", null, null, null);
        if (!NullableParameterKt.preserve(cell).equals("value")) throw new AssertionError();
        cell.value = null;
        if (NullableParameterKt.preserve(cell) != null || NullableParameterKt.nullableView(cell) != null) throw new AssertionError("nullable type parameter");
        try { NullableParameterKt.nonnull(cell); throw new AssertionError("nonnull bound"); }
        catch (NullPointerException expected) {}
        System.out.println("OK");
    }
}

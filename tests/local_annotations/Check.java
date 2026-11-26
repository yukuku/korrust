public final class Check {
    public static void main(String[] args) {
        if (localannotations.UseKt.local() != 6) throw new AssertionError("local declarations and return");
        if (localannotations.UseKt.expression() != 11) throw new AssertionError("expression");
        if (localannotations.UseKt.localFunction() != 8) throw new AssertionError("local function");
        if (localannotations.UseKt.arrayNames() != 9) throw new AssertionError("named array");
        if (localannotations.ShadowKt.shadowed() != 12) throw new AssertionError("custom Suppress");
        System.out.println("OK");
    }
}

public class InlineReturnCheck {
    public static void main(String[] args) {
        if (anonymousinline.InlineReturnsKt.inferred().read() != 6) throw new AssertionError("inferred mutable capture");
        if (anonymousinline.InlineReturnsKt.conditional(true).read() != 7) throw new AssertionError("conditional method return");
        if (anonymousinline.InlineReturnsKt.conditional(false).read() != 9) throw new AssertionError("conditional method continuation");
        System.out.println("OK");
    }
}

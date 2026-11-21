package stable;
import stable.dependency.Box;
public final class Calls {
    private static void eq(int expected, int actual) {
        if (expected != actual) throw new AssertionError("expected=" + expected + " actual=" + actual);
    }
    public static void main(String[] args) {
        Box empty = new Box(null, null);
        Box text = new Box("A\ud83d\ude00", null);
        Box nested = new Box(null, text);
        eq(-1, ModelsKt.guarded(empty));
        eq(3, ModelsKt.guarded(text));
        eq(3, ModelsKt.reversed(text));
        eq(-1, ModelsKt.complement(empty));
        eq(3, ModelsKt.complement(text));
        eq(-1, ModelsKt.early(empty));
        eq(3, ModelsKt.early(text));
        eq(-1, ModelsKt.conjunction(empty));
        eq(3, ModelsKt.conjunction(nested));
        eq(3, ModelsKt.chain(nested));
        eq(-1, ModelsKt.shadow(text, empty));
        eq(3, ModelsKt.shadow(text, text));
        System.out.println("OK");
    }
}

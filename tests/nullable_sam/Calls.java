public final class Calls {
    private static void equal(Object actual, Object expected) { if (!actual.equals(expected)) throw new AssertionError(actual + " != " + expected); }
    public static void main(String[] args) {
        equal(EffectsKt.direct(), 7); equal(EffectsKt.labeled(), 12); equal(EffectsKt.capture(), 14);
        equal(EffectsKt.absent(), -1); equal(EffectsKt.constructor(), 13); equal(EffectsKt.generic(), "ok!");
        equal(EffectsKt.nullableArgument(null), 8); equal(EffectsKt.nullableArgument("abc"), 3); equal(EffectsKt.ordinaryLiteral(), 6);
        equal(EffectsKt.kotlinCallback(), 9);
        try { EffectsKt.callbackThrows(); throw new AssertionError(); }
        catch (IllegalStateException expected) { equal(expected.getMessage(), "callback"); }
        System.out.println("OK");
    }
}

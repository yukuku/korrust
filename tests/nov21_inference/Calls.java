package inference;
import java.util.Arrays;
public final class Calls {
    private static void eq(Object expected, Object actual) {
        if (!java.util.Objects.equals(expected, actual)) throw new AssertionError("expected="+expected+" actual="+actual);
    }
    public static void main(String[] args) {
        eq(0, ModelsKt.early(true).length);
        eq(String.class, ModelsKt.early(true).getClass().getComponentType());
        eq("present", ModelsKt.early(false)[0]);
        for (String value : new String[]{null, "", "abc", "\ud83d\ude00"}) {
            int length = value == null ? -1 : value.length();
            eq(length, ModelsKt.guarded(value));
            eq(length, ModelsKt.reversed(value));
            eq(length, ModelsKt.receiver(value));
            eq(length, ModelsKt.selection(value));
            eq(length, ModelsKt.nullableEquality(value));
            eq(length, ModelsKt.nullableFalseEquality(value));
            eq(length, ModelsKt.directBoolean(value));
            eq(length, ModelsKt.importedAlias(value));
            eq(length, ModelsKt.equalsTrue(value));
            eq(length, ModelsKt.equalsTrueReverse(value));
            eq(length, ModelsKt.notEqualsFalse(value));
            eq(length, ModelsKt.notEqualsFalseReverse(value));
            eq(length, ModelsKt.equalsFalse(value));
            eq(length, ModelsKt.equalsFalseReverse(value));
            eq(length, ModelsKt.notEqualsTrue(value));
            eq(length, ModelsKt.notEqualsTrueReverse(value));
            eq(length, ModelsKt.nullableEqualityReverse(value));
            eq(length, ModelsKt.nullableFalseEqualityReverse(value));
            eq(length, ModelsKt.shortCircuitAnd(value));
            int nonempty = value == null || value.isEmpty() ? -1 : value.length();
            eq(nonempty, ModelsKt.standard(value));
            eq(nonempty, ModelsKt.standardBoolean(value));
            eq(nonempty, ModelsKt.shortCircuitOr(value));
        }
        eq(3, ModelsKt.typeResult("abc"));
        eq(-1, ModelsKt.typeResult(Integer.valueOf(8)));
        eq(-1, ModelsKt.typeResult(null));
        eq(3, ModelsKt.both("a", "bc"));
        eq(-1, ModelsKt.both(null, "b"));
        eq(0, ModelsKt.initialized(true).length);
        eq("value", ModelsKt.initialized(false)[0]);
        eq(0, ModelsKt.caught(true).length);
        eq("value", ModelsKt.caught(false)[0]);
        eq(java.util.regex.Matcher.class, ModelsKt.matchers().getClass().getComponentType());
        eq(2, ModelsKt.matchers().length);
        java.util.regex.Matcher[] matchers = ModelsKt.matchers();
        eq(true, matchers[0] != matchers[1]);
        eq(null, ModelsKt.bound(""));
        eq("abc", ModelsKt.bound("abc"));
        eq(null, ModelsKt.standardBound(""));
        eq("abc", ModelsKt.standardBound("abc"));
        try { ModelsKt.contextExtension("message"); throw new AssertionError(); }
        catch (IllegalArgumentException expected) { eq("message", expected.getMessage()); }
        eq("12", ModelsKt.nullableFunction(12));
        eq("34", ModelsKt.nullableFunctionRef(34));
        eq(null, ModelsKt.absentFunction(8));
        eq("item item", ModelsKt.nullableJoined(Arrays.asList((byte) 1, (byte) 2)));
        eq("item item", ModelsKt.nullableJoinedFull(Arrays.asList((byte) 1, (byte) 2)));
        eq(Boolean.FALSE, ModelsKt.ordinaryAny("value"));
        eq(true, ModelsKt.ordinaryAny(null).equals(Boolean.FALSE));
        eq(false, Boolean.FALSE.equals(ModelsKt.ordinaryAny(null)));
        System.out.println("OK");
    }
}

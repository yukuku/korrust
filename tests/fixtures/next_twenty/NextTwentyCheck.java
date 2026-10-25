import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import next.probes.*;
import next.support.Tracker;

public final class NextTwentyCheck {
    static int checks;
    static void yes(boolean value) { checks++; if (!value) throw new AssertionError("check " + checks); }
    static void eq(long actual, long expected) { checks++; if (actual != expected) throw new AssertionError(actual + " != " + expected + " at " + checks); }
    static void same(float actual, float expected) { checks++; if (Float.floatToIntBits(actual) != Float.floatToIntBits(expected)) throw new AssertionError(actual + " != " + expected + " at " + checks); }
    static void throwsType(Class<? extends Throwable> type, Runnable action) { checks++; try { action.run(); } catch (Throwable error) { if (type.isInstance(error)) return; throw new AssertionError("expected " + type + ", received " + error, error); } throw new AssertionError("expected " + type); }
    public static void main(String[] args) {
        String id = args[0];
        switch (id) {
            case "inclusive_ranges": {
                eq(InclusiveKt.inclusiveCount(1, 4), 4); eq(InclusiveKt.inclusiveCount(4, 1), 0);
                eq(InclusiveKt.inclusiveCount(Integer.MAX_VALUE, Integer.MAX_VALUE), 1);
                eq(InclusiveKt.inclusiveCount(Integer.MAX_VALUE - 1, Integer.MAX_VALUE), 2);
                eq(InclusiveKt.inclusiveCount(Integer.MIN_VALUE, Integer.MIN_VALUE), 1);
                eq(InclusiveKt.inclusiveFlow(), 8); Tracker.reset(); eq(InclusiveKt.inclusiveOnce(), 6); eq(Tracker.calls, 2); yes(Tracker.events.equals("13")); break;
            }
            case "descending_progressions": {
                eq(DescendingKt.descendingCount(4, 1), 4); eq(DescendingKt.descendingCount(1, 4), 0);
                eq(DescendingKt.descendingCount(Integer.MIN_VALUE, Integer.MIN_VALUE), 1);
                eq(DescendingKt.descendingCount(Integer.MIN_VALUE + 1, Integer.MIN_VALUE), 2);
                eq(DescendingKt.descendingCount(Integer.MAX_VALUE, Integer.MAX_VALUE), 1);
                eq(DescendingKt.ordinaryDownCount(3, 1), 3); eq(DescendingKt.customStrideSum(1, 3), 6); eq(DescendingKt.sourceInfixInteger(), 7); eq(DescendingKt.sourceMemberInteger(new InfixBox(2)), 7);
                eq(DescendingKt.libraryInfixInteger(), 34); yes(DescendingKt.libraryInfixText().equals("ab")); eq(DescendingKt.memberInfixPriority(), 5); eq(DescendingKt.memberOrdinaryPriority(), 5); eq(DescendingKt.memberLengthPriority("abc"), 3); yes(DescendingKt.sourceOverridesDefault().equals("source")); break;
            }
            case "stepped_progressions": {
                eq(SteppedKt.steppedSum(1, 6, 2), 9); eq(SteppedKt.steppedSum(5, 1, 2), 0);
                eq(SteppedKt.descendingStepSum(6, 1, 2), 12); eq(SteppedKt.descendingStepSum(1, 6, 2), 0); eq(SteppedKt.progressionReuse(), 18); eq(SteppedKt.localProgressionNames(), 16);
                eq(SteppedKt.steppedSum(Integer.MAX_VALUE - 1, Integer.MAX_VALUE, 3), Integer.MAX_VALUE - 1);
                eq(SteppedKt.descendingStepSum(Integer.MIN_VALUE + 1, Integer.MIN_VALUE, 3), Integer.MIN_VALUE + 1);
                throwsType(IllegalArgumentException.class, () -> SteppedKt.steppedSum(1, 4, 0));
                throwsType(IllegalArgumentException.class, () -> SteppedKt.steppedSum(4, 1, -1));
                throwsType(IllegalArgumentException.class, () -> SteppedKt.descendingStepSum(1, 4, 0)); break;
            }
            case "array_iteration": {
                eq(ArraysKt.arraySum(new int[]{2, -4, 9}), 7); eq(ArraysKt.arraySum(new int[0]), 0); eq(ArraysKt.byteSum(new byte[]{-128, 0, 127}), -1); eq(ArraysKt.charSum(new char[]{0, 65, 0xd800}), 65 + 0xd800);
                Tracker.reset(); eq(ArraysKt.arrayOnce(new int[]{1, 2, 3}), 6); eq(Tracker.calls, 1); break;
            }
            case "string_iteration": {
                eq(StringsKt.stringSum(""), 0); eq(StringsKt.stringSum("A\u0000\ud83d\ude00"), 65 + 0xd83d + 0xde00);
                Tracker.reset(); eq(StringsKt.stringOnce("ab"), 195); eq(Tracker.calls, 1); break;
            }
            case "list_iteration": {
                eq(ListsKt.listSum(Arrays.asList("a", "bc", "")), 3); eq(ListsKt.listSum(Collections.emptyList()), 0);
                Tracker.reset(); eq(ListsKt.listOnce(Arrays.asList("a", "bc")), 3); eq(Tracker.calls, 1);
                eq(ListsKt.longListSum(Arrays.asList(3L, -1L, Long.MAX_VALUE)), Long.MIN_VALUE + 1); eq(ListsKt.nullableListSum(Arrays.asList(null, "ab", null, "c")), 3);
                eq(ListsKt.iterableSum(Arrays.asList("a", "bc")), 3); eq(ListsKt.nullableLongIterableSum(Arrays.asList(3L, null, -1L)), 2);
                next.support.Words words = new next.support.Words(); eq(ListsKt.customIterableSum(words), 3); eq(words.iteratorCalls, 1);
                Tracker.CountingList list = new Tracker.CountingList(); eq(ListsKt.listSum(list), 3); eq(list.iteratorCalls, 1);
                throwsType(java.util.ConcurrentModificationException.class, () -> ListsKt.listMutation(new java.util.ArrayList<>(Arrays.asList("a", "b")))); break;
            }
            case "indices_iteration": {
                eq(IndicesKt.arrayIndices(new int[]{8, 4, -2}), 0); eq(IndicesKt.arrayIndices(new int[0]), 0);
                eq(IndicesKt.stringIndices("a\u0000\ud800"), 2 * 0xd800); eq(IndicesKt.stringIndices(""), 0);
                eq(IndicesKt.listIndices(Arrays.asList("long", "ab", "xyz")), 8); eq(IndicesKt.listIndices(Collections.emptyList()), 0);
                eq(IndicesKt.extensionProperty("ab"), 12); eq(IndicesKt.extensionProperty(""), 0); eq(IndicesKt.extensionProperty("\ud83d\ude00"), 12); break;
            }
            case "do_while": {
                eq(DoWhileKt.doWhileSum(0), 1); eq(DoWhileKt.doWhileSum(1), 1); eq(DoWhileKt.doWhileSum(3), 4);
                eq(DoWhileKt.doWhileSum(5), 13); eq(DoWhileKt.doWhileBreak(), 3); break;
            }
            case "float_arithmetic": {
                same(FloatArithmeticKt.floatAdd(1.25f, 2.5f), 3.75f); same(FloatArithmeticKt.floatSubtract(1f, 3f), -2f);
                same(FloatArithmeticKt.floatMultiply(-0f, 2f), -0f); same(FloatArithmeticKt.floatDivide(1f, 0f), Float.POSITIVE_INFINITY);
                same(FloatArithmeticKt.floatDivide(0f, 0f), Float.NaN); same(FloatArithmeticKt.floatRemainder(-5.5f, 2f), -1.5f);
                same(FloatArithmeticKt.floatRemainder(5f, 0f), Float.NaN); same(FloatArithmeticKt.floatAdd(Float.MAX_VALUE, Float.MAX_VALUE), Float.POSITIVE_INFINITY); break;
            }
            case "float_comparisons": {
                for (float a : new float[]{Float.NaN, Float.NEGATIVE_INFINITY, -1f, -0f, 0f, 1f, Float.POSITIVE_INFINITY}) {
                    for (float b : new float[]{Float.NaN, Float.NEGATIVE_INFINITY, -1f, -0f, 0f, 1f, Float.POSITIVE_INFINITY}) {
                        yes(FloatComparisonsKt.floatLt(a,b) == (a < b)); yes(FloatComparisonsKt.floatLe(a,b) == (a <= b));
                        yes(FloatComparisonsKt.floatGt(a,b) == (a > b)); yes(FloatComparisonsKt.floatGe(a,b) == (a >= b));
                        yes(FloatComparisonsKt.floatEq(a,b) == (a == b)); yes(FloatComparisonsKt.floatNe(a,b) == (a != b));
                    }
                }
                yes(!FloatComparisonsKt.nullableFloatEq(Float.NaN, Float.NaN));
                yes(FloatComparisonsKt.nullableFloatEq(-0f, 0f));
                yes(FloatComparisonsKt.nullableFloatEq(null, null)); yes(!FloatComparisonsKt.nullableFloatEq(null, 0f)); break;
            }
            case "float_conversions": {
                for (float f : new float[]{Float.NaN, Float.NEGATIVE_INFINITY, Float.POSITIVE_INFINITY, -2.9f, -0f, 0f, 3.9f, Float.MAX_VALUE, Float.MIN_VALUE}) {
                    eq(FloatConversionsKt.floatToInt(f), (int) f); eq(FloatConversionsKt.floatToLong(f), (long) f);
                }
                for (int n : new int[]{Integer.MIN_VALUE, -16777217, 0, 16777217, Integer.MAX_VALUE}) same(FloatConversionsKt.intToFloat(n), (float) n);
                for (long n : new long[]{Long.MIN_VALUE, -16777217L, 0, 16777217L, Long.MAX_VALUE}) same(FloatConversionsKt.longToFloat(n), (float) n); break;
            }
            case "compound_assignments": {
                for (int n : new int[]{-11, 0, 7, Integer.MAX_VALUE}) eq(CompoundsKt.compoundInt(n), ((n - 2) * 3 / 2) % 7);
                for (long n : new long[]{-11, 0, 7, Long.MAX_VALUE}) eq(CompoundsKt.compoundLong(n), (((n + 2) - 1) * 3 / 2) % 7);
                for (float n : new float[]{-11.25f, 0f, 7.5f, Float.NaN}) same(CompoundsKt.compoundFloat(n), (((n + 2f) - 1f) * 3f / 2f) % 7f); break;
            }
            case "decrement": {
                eq(DecrementKt.decrement(5), 533); eq(DecrementKt.decrement(0), -22);
                eq(DecrementKt.decrementWrap(Integer.MIN_VALUE), Integer.MAX_VALUE); break;
            }
            case "reference_identity": {
                Object a = new String("same"), b = new String("same");
                yes(IdentityKt.identical(a,a)); yes(!IdentityKt.identical(a,b)); yes(IdentityKt.different(a,b));
                yes(IdentityKt.identical(null,null)); yes(IdentityKt.different(null,a));
                Tracker.reset(); yes(!IdentityKt.identityOrder(null,a)); eq(Tracker.calls, 2); yes(Tracker.events.equals("NV")); break;
            }
            case "structural_equality": {
                yes(Scratch.same()); yes(Scratch.thisSame());
                Tracker.EqualBox a = new Tracker.EqualBox(7), b = new Tracker.EqualBox(7), c = new Tracker.EqualBox(8);
                yes(EqualityKt.equalObjects(a,b)); eq(a.comparisons, 1); eq(b.comparisons, 0);
                yes(EqualityKt.unequalObjects(a,c)); yes(EqualityKt.equalObjects(a,a)); eq(a.comparisons, 3); yes(EqualityKt.equalObjects(null,null)); yes(!EqualityKt.equalObjects(null,a));
                yes(EqualityKt.equalStrings(new String("x"), new String("x"))); yes(EqualityKt.equalStrings(null,null)); yes(!EqualityKt.equalStrings(null,""));
                yes(EqualityKt.equalObjects(Float.NaN, Float.NaN)); yes(!EqualityKt.equalObjects(-0f, 0f));
                Tracker.reset(); yes(!EqualityKt.equalityOrder(null,a)); eq(Tracker.calls, 2); yes(Tracker.events.equals("NV")); break;
            }
            case "type_tests": {
                for (Object value : new Object[]{null, "abc", new StringBuilder("abc"), Integer.valueOf(3)}) {
                    yes(TypeTestsKt.stringTest(value) == (value instanceof String)); yes(TypeTestsKt.notStringTest(value) == !(value instanceof String));
                    yes(TypeTestsKt.sequenceTest(value) == (value instanceof CharSequence));
                    yes(TypeTestsKt.nullableStringTest(value) == (value == null || value instanceof String));
                } break;
            }
            case "checked_casts": {
                String text = new String("abc"); yes(CheckedCastsKt.checkedString(text) == text); yes(CheckedCastsKt.checkedNullableString(text) == text);
                yes(CheckedCastsKt.checkedNullableString(null) == null);
                throwsType(NullPointerException.class, () -> CheckedCastsKt.checkedString(null));
                throwsType(ClassCastException.class, () -> CheckedCastsKt.checkedString(Integer.valueOf(3)));
                throwsType(ClassCastException.class, () -> CheckedCastsKt.checkedNullableString(Integer.valueOf(3)));
                eq(CheckedCastsKt.checkedInt(Integer.valueOf(7)), 7); throwsType(ClassCastException.class, () -> CheckedCastsKt.checkedInt(Long.valueOf(7)));
                throwsType(NullPointerException.class, () -> CheckedCastsKt.checkedInt(null)); break;
            }
            case "safe_casts": {
                String text = new String("abc"); yes(SafeCastsKt.safeString(text) == text); yes(SafeCastsKt.safeString(null) == null);
                yes(SafeCastsKt.safeString(Integer.valueOf(3)) == null); Tracker.reset(); yes(SafeCastsKt.safeOnce(text) == text); eq(Tracker.calls, 1);
                eq(SafeCastsKt.safeInt(Integer.valueOf(7)), 7); yes(SafeCastsKt.safeInt(Long.valueOf(7)) == null); yes(SafeCastsKt.safeInt(null) == null); break;
            }
            case "type_smartcasts": {
                eq(SmartcastsKt.smartLength("abcd"), 4); eq(SmartcastsKt.smartLength(null), -1); eq(SmartcastsKt.smartLength(Integer.valueOf(3)), -1);
                eq(SmartcastsKt.smartElse("abc"), 3); eq(SmartcastsKt.smartElse(null), -1); eq(SmartcastsKt.smartEarly("abcd"), 4); eq(SmartcastsKt.smartEarly(null), -1); break;
            }
            case "when_type_conditions": {
                eq(WhenTypesKt.whenType("abcd"), 4); eq(WhenTypesKt.whenType(null), -2);
                eq(WhenTypesKt.whenType(Integer.valueOf(3)), -2); eq(WhenTypesKt.whenType(new StringBuilder("x")), -3);
                eq(WhenTypesKt.whenBooleanType("x"), 1); eq(WhenTypesKt.whenBooleanType(3), 0);
                eq(WhenTypesKt.subjectlessType("abcd"), 4); eq(WhenTypesKt.subjectlessType(null), -1); break;
            }
            default: throw new AssertionError(id);
        }
        System.out.println("{\"checks\":" + checks + "}");
    }
}

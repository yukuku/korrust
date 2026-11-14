package iteration;
import kotlin.ranges.*;
public final class Calls {
    private static void eq(Object actual, Object expected) { if (!actual.equals(expected)) throw new AssertionError(actual + " != " + expected); }
    public static void main(String[] args) {
        Trace.log = ""; eq(ModelsKt.custom(), 2); eq(Trace.log, "fihnhnhnhn");
        eq(ModelsKt.destructured(), 66); eq(ModelsKt.nullableElements(), "nullok"); eq(ModelsKt.nestedLabels(), 3); eq(ModelsKt.initializerLabel(true), 3); eq(ModelsKt.initializerLabel(false), 3);
        try { ModelsKt.protocolFailure(); throw new AssertionError("missing hasNext failure"); }
        catch (IllegalStateException expected) { eq(expected.getMessage(), "hasNext"); }
        eq(ModelsKt.compiled(), 6L); eq(ModelsKt.extension(), 6L); eq(ModelsKt.sourceExtension(), 6);
        Trace.log = ""; eq(ModelsKt.memberRange(), 6); eq(Trace.log, "rihnhnhnhnh");
        eq(ModelsKt.compiledRange(), 6L); eq(ModelsKt.extensionRange(), 6);
        for (int[] pair : new int[][] {{2,5}, {2,2}, {5,2}, {0,Integer.MIN_VALUE}, {Integer.MIN_VALUE,Integer.MIN_VALUE}, {Integer.MIN_VALUE,Integer.MIN_VALUE+1}, {Integer.MAX_VALUE-1,Integer.MAX_VALUE}}) {
            IntRange range = ModelsKt.intRange(pair[0], pair[1]);
            eq(range.isEmpty(), pair[0] >= pair[1]);
            if (!range.isEmpty()) { eq(range.getFirst(), pair[0]); eq(range.getLast(), pair[1]-1); }
        }
        for (long[] pair : new long[][] {{2,5}, {2,2}, {5,2}, {0,Long.MIN_VALUE}, {Long.MIN_VALUE,Long.MIN_VALUE}, {Long.MIN_VALUE,Long.MIN_VALUE+1}, {Long.MAX_VALUE-1,Long.MAX_VALUE}}) {
            LongRange range = ModelsKt.longRange(pair[0], pair[1]);
            eq(range.isEmpty(), pair[0] >= pair[1]);
            if (!range.isEmpty()) { eq(range.getFirst(), pair[0]); eq(range.getLast(), pair[1]-1); }
        }
        for (char[] pair : new char[][] {{'a','d'}, {'b','b'}, {'d','a'}, {'a',0}, {0,0}, {0,1}, {65534,65535}}) {
            CharRange range = ModelsKt.charRange(pair[0], pair[1]);
            eq(range.isEmpty(), pair[0] >= pair[1]);
            if (!range.isEmpty()) { eq(range.getFirst(), pair[0]); eq(range.getLast(), (char)(pair[1]-1)); }
        }
        if (ModelsKt.intRange(1, Integer.MIN_VALUE) != ModelsKt.intRange(2, Integer.MIN_VALUE)) throw new AssertionError("IntRange.EMPTY identity");
        if (ModelsKt.longRange(1, Long.MIN_VALUE) != ModelsKt.longRange(2, Long.MIN_VALUE)) throw new AssertionError("LongRange.EMPTY identity");
        if (ModelsKt.charRange('a', (char)0) != ModelsKt.charRange('b', (char)0)) throw new AssertionError("CharRange.EMPTY identity");
        Trace.log = ""; eq(ModelsKt.rangeEvaluation().toString(), "2..4"); eq(Trace.log, "ab");
        eq(ModelsKt.intSum(2,5), 9); eq(ModelsKt.intSum(0,Integer.MIN_VALUE), 0);
        eq(ModelsKt.longSum(2,5), 9L); eq(ModelsKt.longSum(0,Long.MIN_VALUE), 0L);
        eq(ModelsKt.charSum('a','d'), 294); eq(ModelsKt.charSum('a',(char)0), 0);
        System.out.println("iteration-ok");
    }
}

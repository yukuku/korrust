import java.util.Arrays;
import java.util.Random;
import yuku.alkitab.base.util.IntArrayListUtilKt;
import yuku.alkitab.util.IntArrayList;

public final class AndroidBibleJarCheck {
    static int checks;
    static String events = "";
    static void eq(Object actual, Object expected) {
        checks++;
        if (!java.util.Objects.equals(actual, expected)) throw new AssertionError("expected " + expected + ", got " + actual);
    }
    static void fails(Class<? extends Throwable> kind, Runnable action) {
        checks++;
        try { action.run(); }
        catch (Throwable error) { if (kind.isInstance(error)) return; throw new AssertionError(kind.getName(), error); }
        throw new AssertionError("missing " + kind.getName());
    }
    static final class TracedList extends IntArrayList {
        final int count;
        final boolean nullBuffer;
        TracedList(int count, boolean nullBuffer) { super(4); this.count = count; this.nullBuffer = nullBuffer; add(9); }
        @Override public int size() { events += "S"; return count; }
        @Override public int[] buffer() { events += "B"; return nullBuffer ? null : super.buffer(); }
    }
    static void check(int capacity, int[] expected) {
        IntArrayList list = new IntArrayList(capacity);
        for (int value : expected) list.add(value);
        int[] first = IntArrayListUtilKt.toIntArray(list), second = IntArrayListUtilKt.toIntArray(list);
        eq(Arrays.equals(first, expected), true); eq(Arrays.equals(second, expected), true);
        eq(first == second, false); eq(first == list.buffer(), false);
        if (expected.length != 0) {
            first[0] ^= 0x55555555;
            eq(list.get(0), expected[0]); eq(second[0], expected[0]);
            list.set(0, expected[0] ^ 0x33333333); eq(second[0], expected[0]);
        }
        list.clear(); eq(IntArrayListUtilKt.toIntArray(list).length, 0);
    }
    public static void main(String[] args) {
        Random random = new Random(849138);
        for (int capacity : new int[] {0, 1, 16, 300}) {
            for (int count : new int[] {0, 1, 2, 17, 80}) {
                int[] values = new int[count]; for (int i = 0; i < count; i++) values[i] = i * 31 - count;
                check(capacity, values);
            }
        }
        for (int sample = 0; sample < 300; sample++) {
            int[] values = new int[random.nextInt(120)]; for (int i = 0; i < values.length; i++) values[i] = random.nextInt();
            check(random.nextInt(150), values);
        }
        fails(NullPointerException.class, () -> IntArrayListUtilKt.toIntArray(null));
        events = ""; fails(NegativeArraySizeException.class, () -> IntArrayListUtilKt.toIntArray(new TracedList(-1, false))); eq(events, "S");
        events = ""; fails(NullPointerException.class, () -> IntArrayListUtilKt.toIntArray(new TracedList(1, true))); eq(events, "SB");
        events = ""; eq(Arrays.toString(IntArrayListUtilKt.toIntArray(new TracedList(1, false))), "[9]"); eq(events, "SB");
        System.out.println("{\"checks\":" + checks + "}");
    }
}

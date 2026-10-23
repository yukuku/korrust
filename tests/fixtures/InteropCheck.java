import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Random;
import korrust.probes.InteropKt;
import yuku.alkitab.base.util.IntArrayListUtilKt;
import yuku.alkitab.util.IntArrayList;

public final class InteropCheck {
    private static final MessageDigest DIGEST;
    private static int checks;
    private static String trace = "";
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception error) { throw new ExceptionInInitializerError(error); }
    }
    private static void record(Object actual, Object expected) {
        if (!actual.equals(expected)) throw new AssertionError("Check " + checks + ": " + actual + " != " + expected);
        byte[] bytes = actual.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8);
        for (int shift = 24; shift >= 0; shift -= 8) DIGEST.update((byte)(bytes.length >>> shift));
        DIGEST.update(bytes);
        checks++;
    }
    private static String failure(Runnable action) {
        try { action.run(); return "success"; }
        catch (RuntimeException error) { return error.getClass().getName(); }
    }
    private static String nullMessage(Runnable action) {
        try { action.run(); return "success"; }
        catch (NullPointerException error) { return String.valueOf(error.getMessage()); }
    }
    private static final class TracedList extends IntArrayList {
        private final String name;
        private final boolean nullBuffer;
        private final int size;
        TracedList(String name, boolean nullBuffer, int size) {
            super(8);
            this.name = name;
            this.nullBuffer = nullBuffer;
            this.size = size;
            super.add(0); super.add(0); super.add(1);
        }
        @Override public int size() { trace += name + ".size;"; return size; }
        @Override public int[] buffer() { trace += name + ".buffer;"; return nullBuffer ? null : super.buffer(); }
        @Override public int get(int index) { trace += name + ".get" + index + ";"; return super.get(index); }
        @Override public void clear() { trace += name + ".clear;"; super.clear(); }
    }
    private static void nullFlow(String name, Runnable action) {
        trace = "";
        String result = failure(action);
        record(result, "java.lang.NullPointerException");
        String observed = name + ":" + trace;
        record(observed, observed);
    }
    private static void checkList(IntArrayList list, int[] expected) {
        record(InteropKt.listSize(list), expected.length);
        int[] first = IntArrayListUtilKt.toIntArray(list);
        int[] second = IntArrayListUtilKt.toIntArray(list);
        record(Arrays.toString(first), Arrays.toString(expected));
        record(Arrays.toString(second), Arrays.toString(expected));
        record(first == second, false);
        record(first == list.buffer(), false);
        record(InteropKt.listBuffer(list) == list.buffer(), true);
        record(InteropKt.explicitSize(list), expected.length);
        if (expected.length > 0) {
            int before = expected[0];
            first[0] ^= 0x55555555;
            record(list.get(0), before);
            record(second[0], before);
            list.set(0, before ^ 0x33333333);
            record(second[0], before);
            record(first[0], before ^ 0x55555555);
            list.set(0, before);
        }
    }
    private static void copyCase(int[] source, int[] destination, int offset, int start, int end, boolean overlap) {
        int[] actualSource = source.clone();
        int[] expectedSource = source.clone();
        int[] actualDestination = overlap ? actualSource : destination.clone();
        int[] expectedDestination = overlap ? expectedSource : destination.clone();
        String expectedFailure = failure(() -> {
            System.arraycopy(expectedSource, start, expectedDestination, offset, end - start);
        });
        final int[][] result = new int[1][];
        String actualFailure = failure(() -> result[0] = InteropKt.copy(actualSource, actualDestination, offset, start, end));
        record(actualFailure, expectedFailure);
        record(Arrays.toString(actualDestination), Arrays.toString(expectedDestination));
        record(Arrays.toString(actualSource), Arrays.toString(expectedSource));
        if (actualFailure.equals("success")) record(result[0] == actualDestination, true);
    }
    public static void main(String[] args) {
        for (int capacity : new int[] {0, 1, 16, 300}) {
            IntArrayList list = new IntArrayList(capacity);
            checkList(list, new int[0]);
            for (int count : new int[] {1, 2, 17, 80}) {
                list.clear();
                int[] values = new int[count];
                for (int i = 0; i < count; i++) { values[i] = i * 31 - count; list.add(values[i]); }
                checkList(list, values);
            }
            InteropKt.mutate(list, 987);
            record(list.get(0), 987);
            record(list.get(list.size() - 1), 987);
            InteropKt.emptyList(list);
            record(list.size(), 0);
            checkList(list, new int[0]);
        }
        Random random = new Random(849138);
        for (int sample = 0; sample < 300; sample++) {
            IntArrayList list = new IntArrayList(random.nextInt(150));
            int[] values = new int[random.nextInt(120)];
            for (int i = 0; i < values.length; i++) { values[i] = random.nextInt(); list.add(values[i]); }
            checkList(list, values);
        }
        for (int size : new int[] {0, 1, 24}) record(Arrays.toString(InteropKt.allocate(size)), Arrays.toString(new int[size]));
        for (int size : new int[] {-1, Integer.MIN_VALUE}) record(failure(() -> InteropKt.allocate(size)), "java.lang.NegativeArraySizeException");
        int[] source = {1, 2, 3, 4};
        for (int offset = -1; offset <= 5; offset++) {
            for (int start = -1; start <= 5; start++) {
                for (int end = -1; end <= 5; end++) {
                    copyCase(source, new int[4], offset, start, end, false);
                    copyCase(source, source, offset, start, end, true);
                }
            }
        }
        copyCase(source, source, Integer.MAX_VALUE, 0, 0, true);
        copyCase(source, source, 0, Integer.MIN_VALUE, Integer.MAX_VALUE, true);
        record(nullMessage(() -> IntArrayListUtilKt.toIntArray(null)), "Parameter specified as non-null is null: method yuku.alkitab.base.util.IntArrayListUtilKt.toIntArray, parameter <this>");
        record(nullMessage(() -> InteropKt.listSize(null)), "Parameter specified as non-null is null: method korrust.probes.InteropKt.listSize, parameter list");
        record(nullMessage(() -> InteropKt.explicitSize(null)), "Parameter specified as non-null is null: method korrust.probes.InteropKt.explicitSize, parameter <this>");
        TracedList missing = new TracedList("missing", true, 1);
        TracedList other = new TracedList("other", false, 1);
        nullFlow("original", () -> IntArrayListUtilKt.toIntArray(missing));
        nullFlow("return", () -> InteropKt.listBuffer(missing));
        nullFlow("inferred", () -> InteropKt.inferredBuffer(missing, other));
        nullFlow("explicit", () -> InteropKt.explicitBuffer(missing, other));
        nullFlow("copy", () -> InteropKt.copiedBuffer(missing, other));
        nullFlow("size", () -> InteropKt.bufferSize(missing));
        TracedList negative = new TracedList("negative", false, -1);
        nullFlow("direct-before-allocation", () -> InteropKt.directCopyOrder(missing, negative));
        nullFlow("inferred-before-allocation", () -> InteropKt.inferredCopyOrder(missing, negative));
        record(nullMessage(() -> InteropKt.listBuffer(missing)), "buffer(...) must not be null");
        trace = "";
        record(failure(() -> IntArrayListUtilKt.toIntArray(new TracedList("negative", false, -1))), "java.lang.NegativeArraySizeException");
        record(trace, "negative.size;");
        trace = "";
        record(Arrays.toString(InteropKt.orderedCopy(new TracedList("source", false, 1), new TracedList("destination", false, 1), new TracedList("indices", false, 1))), "[0, 0, 1, 0, 0, 0, 0, 0]");
        record(trace, "source.buffer;destination.buffer;indices.get0;indices.get1;indices.get2;");
        nullFlow("ordered", () -> InteropKt.orderedCopy(missing, other, other));
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

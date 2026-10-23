import yuku.alkitab.model.SingleChapterVerses;

public final class ModelCheck {
    private static final class Typed implements SingleChapterVerses.WithTextSizeMult {
        public int getVerseCount() { return 2; }
        public String getVerse(int index) { return new String[] {null, "second"}[index]; }
        public float getTextSizeMult(int index) { return Float.intBitsToFloat(index); }
    }
    private static final class Overridden implements SingleChapterVerses {
        public int getVerseCount() { return 0; }
        public String getVerse(int index) { return "x"; }
        public String getVerseNumberText(int index) { return "override"; }
    }
    public static void main(String[] args) throws Exception {
        String[] labels = new String[40];
        for (int i = 0; i < labels.length; i++) labels[i] = Integer.toString(i);
        SingleChapterVerses empty = (SingleChapterVerses)InterfaceAssertions.artifacts(SingleChapterVerses.class, "getVerseCount", "getVerseNumberText", "WithTextSizeMult", "getTextSizeMult", "VERSE_NUMBER_STRINGS", "EMPTY", labels, 0, -1);
        InterfaceAssertions.record(empty == SingleChapterVerses.Companion.getEMPTY(), true);
        InterfaceAssertions.record(InterfaceAssertions.failure(() -> SingleChapterVerses.DefaultImpls.getVerseNumberText(null, 0)).startsWith("java.lang.NullPointerException:"), true);
        InterfaceAssertions.record(InterfaceAssertions.failure(() -> SingleChapterVerses.WithTextSizeMult.DefaultImpls.getVerseNumberText(null, 0)).startsWith("java.lang.NullPointerException:"), true);
        Overridden overridden = new Overridden();
        InterfaceAssertions.record(overridden.getVerseNumberText(0), "override");
        InterfaceAssertions.record(SingleChapterVerses.DefaultImpls.getVerseNumberText(overridden, 0), "1");
        InterfaceAssertions.record(InterfaceAssertions.call(SingleChapterVerses.class.getDeclaredMethod("access$getVerseNumberText$jd", SingleChapterVerses.class, int.class), null, overridden, 0), "1");
        Typed typed = new Typed();
        SingleChapterVerses.WithTextSizeMult nested = typed;
        InterfaceAssertions.record(typed.getVerse(0), null);
        InterfaceAssertions.record(typed.getVerse(1), "second");
        for (int index : new int[] {Integer.MIN_VALUE, -100, -2, -1, 0, 1, 37, 38, 39, 40, 32767, Integer.MAX_VALUE}) {
            String exception = InterfaceAssertions.failure(() -> empty.getVerse(index));
            InterfaceAssertions.record(exception, "java.lang.IllegalArgumentException:@@getVerse requesting verse_0=" + index + " but verseCount=0");
            int one = index + 1;
            String[] expected = new String[1], actual = new String[1];
            String failure = InterfaceAssertions.failure(() -> expected[0] = one < 40 ? labels[one] : Integer.toString(one));
            String observed = InterfaceAssertions.failure(() -> actual[0] = typed.getVerseNumberText(index));
            InterfaceAssertions.record(observed, failure);
            if (failure.equals("success")) {
                InterfaceAssertions.record(actual[0], expected[0]);
                InterfaceAssertions.record(empty.getVerseNumberText(index), expected[0]);
                InterfaceAssertions.record(SingleChapterVerses.DefaultImpls.getVerseNumberText(typed, index), expected[0]);
                InterfaceAssertions.record(SingleChapterVerses.WithTextSizeMult.DefaultImpls.getVerseNumberText(nested, index), expected[0]);
            }
            InterfaceAssertions.record(Float.floatToRawIntBits(nested.getTextSizeMult(index)), index);
        }
        InterfaceAssertions.finish();
    }
}

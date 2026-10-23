import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Objects;
import java.util.Random;
import kotlin.Metadata;
import yuku.alkitab.model.SingleChapterVerses;
import yuku.alkitab.yes2.Yes2SingleChapterVerses;

public final class Yes2Check {
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception error) { throw new ExceptionInInitializerError(error); }
    }
    private static void record(Object actual, Object expected) {
        if (!Objects.equals(actual, expected)) throw new AssertionError("Check " + checks + ": " + actual + " != " + expected);
        byte[] bytes = String.valueOf(actual).getBytes(java.nio.charset.StandardCharsets.UTF_8);
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
    private static void checkArray(String[] source, Field field) throws Exception {
        Yes2SingleChapterVerses first = new Yes2SingleChapterVerses(source);
        Yes2SingleChapterVerses second = new Yes2SingleChapterVerses(source);
        SingleChapterVerses throughInterface = first;
        record(first == second, false);
        record(field.get(first) == source, true);
        record(field.get(second) == source, true);
        record(first.getVerseCount(), source.length);
        record(throughInterface.getVerseCount(), source.length);
        for (int i = 0; i < source.length; i++) {
            record(first.getVerse(i), source[i]);
            record(throughInterface.getVerse(i) == source[i], true);
            String changed = i % 3 == 0 ? null : new String("changed:" + i);
            source[i] = changed;
            record(first.getVerse(i) == changed, true);
            record(second.getVerse(i) == changed, true);
            record(first.getVerseCount(), source.length);
        }
        for (int index : new int[] {Integer.MIN_VALUE, -1, source.length, Integer.MAX_VALUE}) {
            record(failure(() -> first.getVerse(index)), failure(() -> { String ignored = source[index]; }));
            record(failure(() -> throughInterface.getVerse(index)), "java.lang.ArrayIndexOutOfBoundsException");
        }
    }
    public static void main(String[] args) throws Exception {
        Class<?> type = Yes2SingleChapterVerses.class;
        record(type.getModifiers(), Modifier.PUBLIC | Modifier.FINAL);
        record(type.getSuperclass(), Object.class);
        record(Arrays.asList(type.getInterfaces()), Arrays.asList(SingleChapterVerses.class));
        record(type.getDeclaredConstructors().length, 1);
        record(type.getDeclaredConstructors()[0].getModifiers(), Modifier.PUBLIC);
        record(Arrays.asList(type.getDeclaredConstructors()[0].getParameterTypes()), Arrays.asList(String[].class));
        record(type.getDeclaredFields().length, 1);
        Field verses = type.getDeclaredField("verses");
        record(verses.getModifiers(), Modifier.PRIVATE | Modifier.FINAL);
        record(verses.getType(), String[].class);
        verses.setAccessible(true);
        Metadata metadata = type.getAnnotation(Metadata.class);
        record(metadata != null, true);
        record(metadata.k(), 1);
        record(metadata.d1().length > 0, true);
        record(metadata.d2().length > 0, true);
        record(type.getMethod("getVerseNumberText", int.class).isBridge(), true);
        record(type.getMethod("getVerseNumberText", int.class).isSynthetic(), false);
        record(nullMessage(() -> new Yes2SingleChapterVerses(null)), "Parameter specified as non-null is null: method yuku.alkitab.yes2.Yes2SingleChapterVerses.<init>, parameter verses");
        checkArray(new String[0], verses);
        checkArray(new String[] {null}, verses);
        checkArray(new String[] {"", "plain", "\u0000", "\ud800", "\udc00", "\ud83d\ude00", "漢字", "line\nnext"}, verses);
        Random random = new Random(291774);
        for (int sample = 0; sample < 64; sample++) {
            String[] source = new String[random.nextInt(257)];
            for (int i = 0; i < source.length; i++) source[i] = random.nextInt(4) == 0 ? null : new String(new char[] {(char)random.nextInt(65536), (char)random.nextInt(65536)});
            checkArray(source, verses);
        }
        Yes2SingleChapterVerses chapter = new Yes2SingleChapterVerses(new String[0]);
        SingleChapterVerses interfaceView = chapter;
        String[] numbers = new String[40];
        for (int i = 0; i < numbers.length; i++) numbers[i] = Integer.toString(i);
        for (int index : new int[] {Integer.MIN_VALUE, -2, -1, 0, 1, 37, 38, 39, 40, 999, Integer.MAX_VALUE}) {
            int oneBased = index + 1;
            String[] expected = new String[1];
            String expectedFailure = failure(() -> expected[0] = oneBased < 40 ? numbers[oneBased] : Integer.toString(oneBased));
            String[] actual = new String[1];
            record(failure(() -> actual[0] = chapter.getVerseNumberText(index)), expectedFailure);
            if (expectedFailure.equals("success")) record(actual[0], expected[0]);
            record(failure(() -> actual[0] = interfaceView.getVerseNumberText(index)), expectedFailure);
            if (expectedFailure.equals("success")) record(actual[0], expected[0]);
        }
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

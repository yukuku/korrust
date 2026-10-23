import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import fixtures.WhenTracker;
import korrust.probes.CharactersKt;
import yuku.alkitab.base.widget.VerseNumberPlacementKt;

public final class CharactersCheck {
    private static final MessageDigest DIGEST;
    private static int checks;
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
    private static boolean paragraph(char[] text, int length) {
        int cursor = 2;
        while ((long)length - cursor > 1 && text[cursor] == '@') {
            char marker = text[cursor + 1];
            if ("5679".indexOf(marker) >= 0) { cursor += 2; continue; }
            return "^1234".indexOf(marker) >= 0;
        }
        return false;
    }
    private static void appCase(char[] text, int length) {
        final boolean[] expected = new boolean[1];
        final boolean[] actual = new boolean[1];
        String expectedFailure = failure(() -> expected[0] = paragraph(text, length));
        String actualFailure = failure(() -> actual[0] = VerseNumberPlacementKt.startsWithGutterParagraph(text, length));
        record(actualFailure, expectedFailure);
        if (expectedFailure.equals("success")) record(actual[0], expected[0]);
    }
    private static void intWhen(int subject, int first, int second, int third) {
        WhenTracker tracker = new WhenTracker(subject, first, second, third);
        int expected = subject == first || subject == second ? 11 : subject == third ? 13 : 17;
        String expectedTrace = subject == first ? "S1" : subject == second ? "S12" : "S123";
        record(CharactersKt.intWhen(tracker), expected);
        record(tracker.trace, expectedTrace);
    }
    private static void charWhen(char[] text, int subject, int first, int second, int third) {
        WhenTracker tracker = new WhenTracker(subject, first, second, third);
        char value = text[subject];
        int expected = value == text[first] || value == text[second] ? 11 : value == text[third] ? 13 : 17;
        String expectedTrace = value == text[first] ? "S1" : value == text[second] ? "S12" : "S123";
        record(CharactersKt.charWhen(text, tracker), expected);
        record(tracker.trace, expectedTrace);
    }
    private static void charWhenFailure(char[] text, int subject, int first, int second, int third, String trace) {
        WhenTracker tracker = new WhenTracker(subject, first, second, third);
        record(failure(() -> CharactersKt.charWhen(text, tracker)), "java.lang.ArrayIndexOutOfBoundsException");
        record(tracker.trace, trace);
    }
    public static void main(String[] args) {
        char[] all = new char[65536];
        for (int unit = 0; unit < all.length; unit++) all[unit] = (char)unit;
        record(CharactersKt.charArrayEcho(all) == all, true);
        record(CharactersKt.charSize(all), all.length);
        record(CharactersKt.charSize(new char[0]), 0);
        for (int unit = 0; unit < all.length; unit++) {
            char a = (char)unit, b = (char)(65535 - unit);
            record((int)CharactersKt.charIdentity(a), unit);
            record((int)CharactersKt.charLocal(a), unit);
            record((int)CharactersKt.charRead(all, unit), unit);
            record(CharactersKt.charEqual(a, b), a == b);
            record(CharactersKt.charUnequal(a, b), a != b);
            record(CharactersKt.charLess(a, b), a < b);
            record(CharactersKt.charLessEqual(a, b), a <= b);
            record(CharactersKt.charGreater(a, b), a > b);
            record(CharactersKt.charGreaterEqual(a, b), a >= b);
            record(CharactersKt.charEqual(a, a), true);
            record(CharactersKt.charLessEqual(a, a), true);
            record(CharactersKt.charGreaterEqual(a, a), true);
            appCase(new char[] {'@', '@', '@', a}, 4);
        }
        int[] literals = {0, 0xd800, 0xdfff, 9, 8, 10, 13, 39, 34, 92, 36, 36, 0x6f22, 65535};
        for (int i = 0; i < literals.length; i++) record((int)CharactersKt.charLiteral(i), literals[i]);
        for (int index : new int[] {-1, 65536, Integer.MIN_VALUE, Integer.MAX_VALUE}) {
            record(failure(() -> CharactersKt.charRead(all, index)), "java.lang.ArrayIndexOutOfBoundsException");
        }
        record(nullMessage(() -> CharactersKt.charArrayEcho(null)), "Parameter specified as non-null is null: method korrust.probes.CharactersKt.charArrayEcho, parameter text");
        record(nullMessage(() -> CharactersKt.charSize(null)), "Parameter specified as non-null is null: method korrust.probes.CharactersKt.charSize, parameter text");
        record(nullMessage(() -> CharactersKt.charRead(null, 0)), "Parameter specified as non-null is null: method korrust.probes.CharactersKt.charRead, parameter text");
        record(nullMessage(() -> VerseNumberPlacementKt.startsWithGutterParagraph(null, 0)), "Parameter specified as non-null is null: method yuku.alkitab.base.widget.VerseNumberPlacementKt.startsWithGutterParagraph, parameter text");
        int[] boundaries = {Integer.MIN_VALUE, Integer.MIN_VALUE + 1, -1, 0, 1, Integer.MAX_VALUE - 1, Integer.MAX_VALUE};
        for (int a : boundaries) for (int b : boundaries) for (int c : boundaries) record(CharactersKt.addAssign(a, b, c), a + b + c);
        Random random = new Random(29418);
        for (int i = 0; i < 300; i++) {
            int a = random.nextInt(), b = random.nextInt(), c = random.nextInt();
            record(CharactersKt.addAssign(a, b, c), a + b + c);
        }
        for (int subject : boundaries) for (int first : boundaries) for (int second : boundaries) for (int third : boundaries) intWhen(subject, first, second, third);
        char[] markers = {'\u0000', '@', '5', '1', '\ud800', '\uffff'};
        for (int subject = 0; subject < markers.length; subject++) for (int first = 0; first < markers.length; first++) for (int second = 0; second < markers.length; second++) for (int third = 0; third < markers.length; third++) charWhen(markers, subject, first, second, third);
        WhenTracker shortInt = new WhenTracker(1, 1, 0, 0);
        record(CharactersKt.intWhen(shortInt), 11);
        record(shortInt.trace, "S1");
        WhenTracker shortChar = new WhenTracker(0, 0, -1, -1);
        record(CharactersKt.charWhen(markers, shortChar), 11);
        record(shortChar.trace, "S1");
        charWhenFailure(markers, -1, 0, 1, 2, "S");
        charWhenFailure(markers, 0, -1, 1, 2, "S1");
        charWhenFailure(markers, 0, 1, -1, 2, "S12");
        charWhenFailure(markers, 0, 1, 2, -1, "S123");
        WhenTracker empty = new WhenTracker(0, 0, 0, 0);
        record(CharactersKt.emptyWhen(empty), 29);
        record(empty.trace, "S");
        for (int value = -3; value <= 5; value++) {
            record(CharactersKt.whenScopes(value), value == 0 ? 40 : value == 1 ? 5 : value == 2 ? 9 : 11);
            record(CharactersKt.whenFallthrough(value), value == 1 ? 13 : value == 2 || value == 3 ? 17 : 10);
        }
        for (String styles : new String[] {"", "@5", "@6", "@7", "@9", "@5@6@7@9", "@9@9@5"}) {
            for (String marker : new String[] {"@^", "@1", "@2", "@3", "@4", "@0", "@8", "@x", "text", "@", ""}) {
                for (String prefix : new String[] {"@@", "xx"}) {
                    char[] text = (prefix + styles + marker + "content").toCharArray();
                    for (int length = -2; length <= text.length + 2; length++) appCase(text, length);
                }
            }
        }
        for (char[] text : new char[][] {{}, {'@'}, {'@', '@'}, {'@', '@', '@'}, {'@', '@', '@', '5'}}) {
            for (int length = -2; length <= 8; length++) appCase(text, length);
            appCase(text, Integer.MIN_VALUE);
            appCase(text, Integer.MAX_VALUE);
        }
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

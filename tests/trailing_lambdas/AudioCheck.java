import yuku.alkitab.base.audio.AudioHttpErrorBodyKt;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Collections;
import java.util.Objects;
public final class AudioCheck {
    private static int checks;
    private static void check(byte[] input, String expected) {
        byte[] original = input.clone();
        String actual = AudioHttpErrorBodyKt.formatErrorBody(input);
        if (!Objects.equals(actual, expected)) throw new AssertionError("expected=" + expected + " actual=" + actual);
        if (!Arrays.equals(original, input)) throw new AssertionError("input changed");
        checks++;
    }
    public static void main(String[] args) {
        check(new byte[0], null);
        check(" a\tb\rc\nd ".getBytes(StandardCharsets.US_ASCII), "a b c d");
        check(new byte[]{32, 9, 13, 10}, null);
        String digits = "0123456789abcdef";
        for (int value = 0; value < 256; value++) {
            String expected;
            if (value == 32 || value == 9 || value == 10 || value == 13) expected = null;
            else if (value >= 33 && value <= 126) expected = Character.toString((char) value);
            else expected = "" + digits.charAt(value >>> 4) + digits.charAt(value & 15) + " ...";
            check(new byte[]{(byte) value}, expected);
        }
        for (int size : new int[]{15, 16, 17, 1023}) {
            check(new byte[size], String.join(" ", Collections.nCopies(Math.min(size, 16), "00")) + " ...");
        }
        byte[] text = new byte[1023];
        Arrays.fill(text, (byte) 'x');
        check(text, "x".repeat(1023));
        check(Arrays.copyOf(text, 1024), null);
        check(new byte[1024], null);
        check(new byte[1025], null);
        System.out.println("audio-checks=" + checks);
    }
}

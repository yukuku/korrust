import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import korrust.probes.StringsKt;
import yuku.alkitab.base.audio.AudioErrorCodeKt;

public final class StringsCheck {
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception e) { throw new ExceptionInInitializerError(e); }
    }
    private static void record(Object actual, Object expected) {
        if (!actual.equals(expected)) throw new AssertionError("Mismatch at check " + checks);
        String text = actual.toString();
        int length = text.length();
        for (int shift = 24; shift >= 0; shift -= 8) DIGEST.update((byte)(length >>> shift));
        for (int i = 0; i < length; i++) {
            char unit = text.charAt(i);
            DIGEST.update((byte)(unit >>> 8));
            DIGEST.update((byte)unit);
        }
        checks++;
    }
    private static String failure(Runnable action) {
        try { action.run(); throw new AssertionError("Expected exception"); }
        catch (RuntimeException e) { return e.getClass().getName(); }
    }
    private static String nullMessage(Runnable action) {
        try { action.run(); throw new AssertionError("Expected exception"); }
        catch (NullPointerException e) { return e.getMessage(); }
    }
    private static String random(Random random) {
        char[] units = new char[random.nextInt(15)];
        for (int i = 0; i < units.length; i++) units[i] = (char)random.nextInt(65536);
        return new String(units);
    }
    public static void main(String[] args) {
        record(StringsKt.literal(), "ASCII\u0000é漢😀\uD800\uDC00\uD800X\uDFFF\t\b\n\r\'\"\\$");
        record(StringsKt.internal("soft keyword"), "soft keyword");
        record(StringsKt.dollars(), "$1 $!");
        String[] originals = {"ERROR_CODE_IO_NETWORK_CONNECTION_FAILED", "no_audio_for_version", "ERROR_CODE_IO_ERROR_CODE_X"};
        String[] stripped = {"IO_NETWORK_CONNECTION_FAILED", "no_audio_for_version", "IO_ERROR_CODE_X"};
        for (int i = 0; i < originals.length; i++) record(AudioErrorCodeKt.stripErrorCodePrefix(originals[i]), stripped[i]);
        String[] edges = {"", "ERROR_CODE_", "ERROR_CODE_ERROR_CODE_", "ERROR_CODE", "XERROR_CODE_", "\u0000", "\uD800", "\uDC00", "😀", "é漢"};
        Random random = new Random(724323);
        for (int i = 0; i < 700; i++) {
            String value = i < edges.length ? edges[i] : random(random);
            String other = i % 3 == 0 ? new String(value.toCharArray()) : random(random);
            String prefix = i % 2 == 0 ? value.substring(0, value.length() / 2) : other;
            String suffix = value.substring(value.length() / 2);
            record(StringsKt.identity(value), value);
            record(StringsKt.internalIdentity(value), value);
            record(StringsKt.equal(value, other), value.equals(other));
            record(StringsKt.unequal(value, other), !value.equals(other));
            record(StringsKt.equal(value, new String(value.toCharArray())), true);
            record(StringsKt.equal(value.intern(), new String(value.toCharArray())), true);
            record(StringsKt.length(value), value.length());
            record(StringsKt.empty(value), value.isEmpty());
            record(StringsKt.starts(value, prefix), value.startsWith(prefix));
            record(StringsKt.ends(value, suffix), value.endsWith(suffix));
            record(StringsKt.remove(value, prefix), value.startsWith(prefix) ? value.substring(prefix.length()) : value);
            record(StringsKt.compare(value, other), value.compareTo(other));
            int number = i == 0 ? Integer.MIN_VALUE : i == 1 ? Integer.MAX_VALUE : -i;
            record(StringsKt.concat(value, other, number, i % 2 == 0), value + other + number + (i % 2 == 0));
            record(StringsKt.local("ERROR_CODE_" + value, true), value);
            record(StringsKt.local(value, false), "fallback:" + value);
            String code = i % 2 == 0 ? "ERROR_CODE_" + value : value;
            record(AudioErrorCodeKt.stripErrorCodePrefix(code), code.startsWith("ERROR_CODE_") ? code.substring(11) : code);
            for (int start = 0; start <= value.length(); start++) {
                record(StringsKt.suffix(value, start), value.substring(start));
                for (int end = start; end <= value.length(); end++) record(StringsKt.slice(value, start, end), value.substring(start, end));
            }
        }
        int[][] invalid = {{-1, 0}, {0, 4}, {2, 1}, {4, 4}, {Integer.MIN_VALUE, Integer.MAX_VALUE}};
        for (int[] bounds : invalid) {
            record(failure(() -> StringsKt.slice("abc", bounds[0], bounds[1])), failure(() -> "abc".substring(bounds[0], bounds[1])));
        }
        for (int start : new int[] {-1, 4, Integer.MAX_VALUE}) record(failure(() -> StringsKt.suffix("abc", start)), failure(() -> "abc".substring(start)));
        record(failure(() -> StringsKt.order("abc", -1, 0)), "java.lang.StringIndexOutOfBoundsException");
        record(failure(() -> StringsKt.order("abc", 0, 0)), "java.lang.ArithmeticException");
        record(StringsKt.order("abc", 1, 1), "bc1");
        record(failure(() -> StringsKt.identity(null)), "java.lang.NullPointerException");
        record(failure(() -> StringsKt.internalIdentity(null)), "java.lang.NullPointerException");
        record(failure(() -> AudioErrorCodeKt.stripErrorCodePrefix(null)), "java.lang.NullPointerException");
        record(nullMessage(() -> StringsKt.identity(null)), "Parameter specified as non-null is null: method korrust.probes.StringsKt.identity, parameter value");
        record(nullMessage(() -> StringsKt.internalIdentity(null)), "Parameter specified as non-null is null: method korrust.probes.StringsKt.internalIdentity, parameter value");
        record(nullMessage(() -> AudioErrorCodeKt.stripErrorCodePrefix(null)), "Parameter specified as non-null is null: method yuku.alkitab.base.audio.AudioErrorCodeKt.stripErrorCodePrefix, parameter errorCodeName");
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

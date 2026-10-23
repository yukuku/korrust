import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.CyclicBarrier;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import yuku.alkitab.io.Utf8Decoder;
import yuku.bintex.BintexReader;
import yuku.bintex.BintexWriter;

public final class IoCheck {
    private static final MessageDigest DIGEST;
    private static int checks;
    static {
        try { DIGEST = MessageDigest.getInstance("SHA-256"); }
        catch (Exception error) { throw new ExceptionInInitializerError(error); }
    }
    private interface Action { void run() throws Exception; }
    private static void record(Object actual, Object expected) {
        if (!Objects.equals(actual, expected)) throw new AssertionError("Check " + checks + ": " + actual + " != " + expected);
        String text = String.valueOf(actual);
        for (int i = 0; i < text.length(); i++) { DIGEST.update((byte)(text.charAt(i) >>> 8)); DIGEST.update((byte)text.charAt(i)); }
        DIGEST.update((byte)255);
        checks++;
    }
    private static String failure(Action action, boolean message) {
        try { action.run(); return "success"; }
        catch (Exception error) { return error.getClass().getName() + (message ? ":" + error.getMessage() : ""); }
    }
    private static String decode(byte[] input, int count, boolean lowercase) throws Exception {
        return Utf8Decoder.toStringFromVersesWithPrependedLengths(new BintexReader(new ByteArrayInputStream(input)), count, lowercase);
    }
    private static byte[] verses(String... text) throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        BintexWriter writer = new BintexWriter(output);
        for (String value : text) {
            byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
            writer.writeVarUint(bytes.length);
            output.write(bytes);
        }
        return output.toByteArray();
    }
    private static void equalDecode(byte[] input, String text, String lowercase) {
        record(Utf8Decoder.toString(input), text);
        record(Utf8Decoder.toStringLowerCase(input), lowercase);
        byte[] framed = new byte[input.length + 4];
        System.arraycopy(input, 0, framed, 2, input.length);
        record(Utf8Decoder.toString(framed, 2, input.length), text);
        record(Utf8Decoder.toStringLowerCase(framed, 2, input.length), lowercase);
    }
    public static void main(String[] args) throws Exception {
        Class<?> type = Utf8Decoder.class;
        record(type.getModifiers(), Modifier.PUBLIC | Modifier.FINAL);
        record(type.getField("INSTANCE").get(null) == Utf8Decoder.INSTANCE, true);
        record(type.getDeclaredConstructors().length, 1);
        record(type.getMethod("toString", byte[].class).getAnnotation(kotlin.jvm.JvmStatic.class) != null, true);
        record(type.getDeclaredConstructors()[0].getModifiers(), Modifier.PRIVATE);
        record(Arrays.asList(type.getMethod("toStringFromVersesWithPrependedLengths", BintexReader.class, int.class, boolean.class).getExceptionTypes()), Arrays.asList(IOException.class));
        Field byteField = type.getDeclaredField("byteBuffer"), charField = type.getDeclaredField("charBuffer");
        record(byteField.getGenericType().getTypeName(), "java.lang.ThreadLocal<byte[]>");
        record(charField.getGenericType().getTypeName(), "java.lang.ThreadLocal<char[]>");
        for (Field field : new Field[] {byteField, charField}) {
            record(field.getModifiers(), Modifier.PRIVATE | Modifier.STATIC | Modifier.FINAL);
            field.setAccessible(true);
        }
        @SuppressWarnings("unchecked") ThreadLocal<byte[]> bytes = (ThreadLocal<byte[]>)byteField.get(null);
        @SuppressWarnings("unchecked") ThreadLocal<char[]> chars = (ThreadLocal<char[]>)charField.get(null);
        record(bytes.get().length, 1000);
        record(chars.get().length, 8000);
        record(bytes.get() == bytes.get(), true);
        record(chars.get() == chars.get(), true);
        for (int value = 0; value < 256; value++) {
            byte[] input = {(byte)value};
            if (value >= 0xc2 && value <= 0xf4) {
                record(failure(() -> Utf8Decoder.toString(input), true), "java.lang.IllegalArgumentException:Truncated UTF-8 sequence");
                record(failure(() -> Utf8Decoder.toStringLowerCase(input), true), "java.lang.IllegalArgumentException:Truncated UTF-8 sequence");
            } else {
                char unit = value < 128 ? (char)value : '\ufffd';
                equalDecode(input, String.valueOf(unit), String.valueOf(Character.toLowerCase(unit)));
            }
        }
        int scalars = 0;
        for (int start = 0; start <= Character.MAX_CODE_POINT; start += 4096) {
            StringBuilder text = new StringBuilder(), lower = new StringBuilder();
            for (int cp = start; cp <= Character.MAX_CODE_POINT && cp < start + 4096; cp++) {
                if (cp >= 0xd800 && cp <= 0xdfff) continue;
                text.appendCodePoint(cp);
                lower.appendCodePoint(Character.toLowerCase(cp));
                scalars++;
            }
            equalDecode(text.toString().getBytes(StandardCharsets.UTF_8), text.toString(), lower.toString());
        }
        record(scalars, 1112064);
        for (int[] sequence : new int[][] {{0xc0,0xaf},{0xe0,0x80,0xaf},{0xed,0xa0,0x80},{0xf0,0x80,0x80,0xaf},{0xf4,0x90,0x80,0x80},{0xf5,0x80,0x80,0x80},{0xff}}) {
            byte[] input = new byte[sequence.length + 1];
            for (int i = 0; i < sequence.length; i++) input[i] = (byte)sequence[i];
            input[sequence.length] = 'A';
            equalDecode(input, "\ufffd".repeat(sequence.length) + "A", "\ufffd".repeat(sequence.length) + "a");
        }
        byte[] supplementary = "X𐐀Y".getBytes(StandardCharsets.UTF_8);
        for (int length = 1; length < 4; length++) {
            int slice = length;
            record(failure(() -> Utf8Decoder.toString(supplementary, 1, slice), true), "java.lang.IllegalArgumentException:Truncated UTF-8 sequence");
        }
        for (int start : new int[] {Integer.MIN_VALUE, -1, 0, 1, supplementary.length, Integer.MAX_VALUE}) {
            for (int length : new int[] {Integer.MIN_VALUE, -1, 0, 1, supplementary.length, Integer.MAX_VALUE}) {
                if (start < 0 || length < 0 || start > supplementary.length - length) {
                    record(failure(() -> Utf8Decoder.toString(supplementary, start, length), false), "java.lang.IndexOutOfBoundsException");
                    record(failure(() -> Utf8Decoder.toStringLowerCase(supplementary, start, length), false), "java.lang.IndexOutOfBoundsException");
                }
            }
        }
        record(failure(() -> Utf8Decoder.toString((byte[])null), true), "java.lang.NullPointerException:Parameter specified as non-null is null: method yuku.alkitab.io.Utf8Decoder.toString, parameter ba");
        record(failure(() -> Utf8Decoder.toStringFromVersesWithPrependedLengths(null, 0, false), true), "java.lang.NullPointerException:Parameter specified as non-null is null: method yuku.alkitab.io.Utf8Decoder.toStringFromVersesWithPrependedLengths, parameter br");
        String sample = "İ ΟΣ 𐐀 👩🏽‍💻 𠮷 CAFÉ";
        String lower = "i οσ 𐐨 👩🏽‍💻 𠮷 café";
        byte[] encoded = verses(sample.repeat(1000), "", sample);
        record(decode(encoded, 3, false), sample.repeat(1000) + "\n\n" + sample + "\n");
        record(decode(encoded, 3, true), lower.repeat(1000) + "\n\n" + lower + "\n");
        record(decode(new byte[0], 0, false), "");
        record(decode(new byte[0], -1, true), "");
        ByteArrayOutputStream truncated = new ByteArrayOutputStream();
        BintexWriter writer = new BintexWriter(truncated);
        writer.writeVarUint(4); truncated.write("𐐀".getBytes(StandardCharsets.UTF_8));
        writer.writeVarUint(2); truncated.write(new byte[] {(byte)0xf0,(byte)0x90});
        writer.writeVarUint(1); truncated.write('A');
        record(failure(() -> decode(truncated.toByteArray(), 3, false), true), "java.lang.IllegalArgumentException:Truncated UTF-8 sequence");
        record(failure(() -> decode(truncated.toByteArray(), 3, true), true), "java.lang.IllegalArgumentException:Truncated UTF-8 sequence");
        char[] savedChars = chars.get();
        chars.set(null);
        try { record(failure(() -> Utf8Decoder.toString(new byte[0]), false), "java.lang.NullPointerException"); }
        finally { chars.set(savedChars); }
        byte[] savedBytes = bytes.get();
        bytes.set(null);
        try { record(failure(() -> decode(new byte[0], 0, false), false), "java.lang.NullPointerException"); }
        finally { bytes.set(savedBytes); }
        ExecutorService executor = Executors.newFixedThreadPool(4);
        CyclicBarrier barrier = new CyclicBarrier(4);
        List<Future<Object[]>> futures = new ArrayList<>();
        try {
            for (String text : new String[] {"ABC𐐀", "日本語👩🏽‍💻", "ÉΣ𠮷", "XYZ🌿"}) {
                futures.add(executor.submit(() -> {
                    barrier.await();
                    String content = text.repeat(3000);
                    StringBuilder lowercase = new StringBuilder();
                    content.codePoints().forEach(cp -> lowercase.appendCodePoint(Character.toLowerCase(cp)));
                    byte[] input = content.getBytes(StandardCharsets.UTF_8);
                    byte[] prefixed = verses(content);
                    for (int round = 0; round < 12; round++) {
                        if (!Utf8Decoder.toString(input).equals(content) || !Utf8Decoder.toStringLowerCase(input).equals(lowercase.toString())) throw new AssertionError("Concurrent decoding mismatch");
                        if (!decode(prefixed, 1, false).equals(content + "\n")) throw new AssertionError("Concurrent prefixed decoding mismatch");
                    }
                    return new Object[] {bytes.get(), chars.get()};
                }));
            }
            IdentityHashMap<Object, Boolean> identities = new IdentityHashMap<>();
            for (Future<Object[]> future : futures) for (Object buffer : future.get()) record(identities.put(buffer, true), null);
            record(identities.size(), 8);
        } finally { executor.shutdownNow(); }
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

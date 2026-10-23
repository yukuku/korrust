import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Objects;
import korrust.fixtures.IndexedText;
import korrust.probes.classes.TextStore;

public final class ClassesCheck {
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
    public static void main(String[] args) throws Exception {
        Class<?> type = TextStore.class;
        record(type.getModifiers(), Modifier.PUBLIC | Modifier.FINAL);
        record(Arrays.asList(type.getInterfaces()), Arrays.asList(IndexedText.class));
        record(type.getDeclaredFields().length, 2);
        record(type.getDeclaredConstructors().length, 1);
        record(Arrays.asList(type.getDeclaredConstructors()[0].getParameterTypes()), Arrays.asList(String[].class, int.class));
        record(type.getMethod("label", int.class).isBridge(), true);
        Field elements = type.getDeclaredField("elements");
        Field start = type.getDeclaredField("start");
        record(elements.getModifiers(), Modifier.PRIVATE | Modifier.FINAL);
        record(start.getModifiers(), Modifier.PRIVATE | Modifier.FINAL);
        elements.setAccessible(true);
        start.setAccessible(true);
        for (int length : new int[] {0, 1, 7, 257}) {
            String[] source = new String[length];
            for (int i = 0; i < length; i++) source[i] = i % 2 == 0 ? null : new String("entry:" + i);
            for (int offset : new int[] {Integer.MIN_VALUE, -1, 0, 1, length, Integer.MAX_VALUE}) {
                TextStore store = new TextStore(source, offset);
                IndexedText view = store;
                record(elements.get(store) == source, true);
                record(start.get(store), offset);
                record(store.getItemCount(), length - offset);
                record(view.getItemCount(), length - offset);
                for (int index : new int[] {Integer.MIN_VALUE, -2, -1, 0, 1, length - 1, length, Integer.MAX_VALUE}) {
                    String[] expected = new String[1];
                    String[] actual = new String[1];
                    String expectedFailure = failure(() -> expected[0] = source[index + offset]);
                    record(failure(() -> actual[0] = store.itemAt(index)), expectedFailure);
                    if (expectedFailure.equals("success")) record(actual[0] == expected[0], true);
                    record(failure(() -> actual[0] = view.itemAt(index)), expectedFailure);
                    if (expectedFailure.equals("success")) record(actual[0] == expected[0], true);
                    record(store.label(index), "slot:" + index);
                    record(view.label(index), "slot:" + index);
                }
            }
            TextStore store = new TextStore(source, 0);
            for (int i = 0; i < length; i++) {
                source[i] = new String("mutated:" + i);
                record(store.itemAt(i) == source[i], true);
            }
        }
        record(failure(() -> new TextStore(null, 0)), "java.lang.NullPointerException");
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(DIGEST.digest()) + "\"}");
    }
}

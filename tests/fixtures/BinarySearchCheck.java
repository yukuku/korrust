import java.nio.ByteBuffer;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.Random;
import yuku.alkitab.yes2.util.UnsignedBinarySearchKt;

public final class BinarySearchCheck {
    private static final MessageDigest digest;
    private static int checks;
    static {
        try { digest = MessageDigest.getInstance("SHA-256"); }
        catch (Exception e) { throw new ExceptionInInitializerError(e); }
    }

    private static void check(int[] values, int key) {
        int actual = UnsignedBinarySearchKt.unsignedIntBinarySearch(values, key);
        int insertion = 0;
        while (insertion < values.length && Integer.compareUnsigned(values[insertion], key) < 0) {
            insertion++;
        }
        boolean found = insertion < values.length && values[insertion] == key;
        if (found) {
            if (actual < 0 || actual >= values.length || values[actual] != key) {
                throw new AssertionError("Missing key " + Integer.toUnsignedString(key));
            }
        } else if (actual != -(insertion + 1)) {
            throw new AssertionError("Wrong insertion point: " + actual + " expected " + -(insertion + 1));
        }
        digest.update(ByteBuffer.allocate(8).putInt(key).putInt(actual).array());
        checks++;
    }

    private static void checkArray(int[] values, Random random) {
        int[] boundaries = {0, 1, 2, Integer.MAX_VALUE - 1, Integer.MAX_VALUE, Integer.MIN_VALUE, Integer.MIN_VALUE + 1, -2, -1};
        for (int key : boundaries) { check(values, key); }
        for (int key : values) { check(values, key); }
        for (int i = 0; i < 64; i++) { check(values, random.nextInt()); }
    }

    public static void main(String[] args) {
        Random random = new Random(20261023L);
        int[][] edgeCases = {
            {}, {0}, {-1}, {Integer.MIN_VALUE}, {Integer.MAX_VALUE},
            {0, 1, Integer.MAX_VALUE, Integer.MIN_VALUE, -2, -1},
            {0, 0, 0, Integer.MIN_VALUE, Integer.MIN_VALUE, -1, -1}
        };
        for (int[] values : edgeCases) { checkArray(values, random); }
        for (int n = 0; n < 512; n++) {
            Integer[] boxed = new Integer[random.nextInt(129)];
            for (int i = 0; i < boxed.length; i++) {
                boxed[i] = n % 2 == 0 ? random.nextInt() : random.nextInt(9) - 4;
            }
            Arrays.sort(boxed, Integer::compareUnsigned);
            int[] values = Arrays.stream(boxed).mapToInt(Integer::intValue).toArray();
            checkArray(values, random);
        }
        try {
            UnsignedBinarySearchKt.unsignedIntBinarySearch(null, 0);
            throw new AssertionError("Null parameter accepted");
        } catch (NullPointerException expected) {
            digest.update(expected.getMessage().getBytes(java.nio.charset.StandardCharsets.UTF_8));
            checks++;
        }
        System.out.println("{\"checks\":" + checks + ",\"sha256\":\"" + HexFormat.of().formatHex(digest.digest()) + "\"}");
    }
}

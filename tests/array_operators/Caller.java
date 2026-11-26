import java.util.Arrays;
import arrays.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        byte[] left = {1, 2};
        byte[] result = ConsumerKt.append(left, new byte[]{3});
        if (!Arrays.equals(result, new byte[]{1, 2, 3}) || result == left) throw new AssertionError();
        String[] words = ConsumerKt.strings(new String[]{"a"}, "b");
        if (!Arrays.equals(words, new String[]{"a", "b"})) throw new AssertionError();
        if (ConsumerKt.custom(new int[]{8, 13}, 1) != 13) throw new AssertionError();
    }
}

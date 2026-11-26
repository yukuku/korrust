import java.util.ArrayList;
import java.util.List;
import listops.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        List<String> left = new ArrayList<>(List.of("first"));
        List<String> out = ConsumerKt.concatenate(left, List.of("second", "third"), "last");
        if (!out.equals(List.of("first", "second", "third", "last")) || left.size() != 1)
            throw new AssertionError(out);
        List<Integer> original = new ArrayList<>(List.of(1, 2, 1, 3));
        if (!ConsumerKt.without(original, List.of(1, 3)).equals(List.of(2)) || original.size() != 4)
            throw new AssertionError();
        if (ConsumerKt.custom(original, 3) != 12) throw new AssertionError();
    }
}

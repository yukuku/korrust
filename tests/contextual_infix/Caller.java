import consumers.ConsumerKt;
import java.util.List;
import java.util.Map;

public class Caller {
    public static void main(String[] args) {
        if (!ConsumerKt.inferred().getValue().isEmpty()) throw new AssertionError();
        Map<String, List<String>> v = ConsumerKt.values();
        if (!List.copyOf(v.keySet()).equals(List.of("a", "n", "e", "b"))) throw new AssertionError();
        if (v.get("n") != null || !v.get("e").isEmpty() || !v.get("b").equals(List.of("two"))) throw new AssertionError();
    }
}

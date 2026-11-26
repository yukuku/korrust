import consumers.ConsumerKt;
import dep.Entry;
import java.util.Map;

public class Caller {
    @SuppressWarnings("unchecked")
    public static void main(String[] args) {
        if (ConsumerKt.inferred(new Entry<>("a", 4L), new Entry<>("b", 9L)) != 9L) throw new AssertionError();
        Entry<Integer, String>[] rest = new Entry[]{new Entry<>(2, "last")};
        if (!ConsumerKt.spread(new Entry<>(1, "first"), rest).equals("last")) throw new AssertionError();
        if (!ConsumerKt.words().equals(Map.of("a", "one", "b", "two"))) throw new AssertionError();
        if (!ConsumerKt.optional().get("a").equals(java.util.List.of("one"))) throw new AssertionError();
        if (!ConsumerKt.emptyValue().containsKey("none") || ConsumerKt.emptyValue().get("none") != null) throw new AssertionError();
    }
}

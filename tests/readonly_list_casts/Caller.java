import java.util.List;
public class Caller {
    public static void main(String[] args) {
        List<?> ints = List.of(2);
        if (ConsumerKt.checked(ints) != ints || ConsumerKt.safe(ints) != ints || ConsumerKt.safe(2) != null || ConsumerKt.safe(null) != null) throw new AssertionError();
        if (!"ok".equals(ConsumerKt.first(List.of("ok")))) throw new AssertionError();
        try { ConsumerKt.first(ints); throw new AssertionError(); } catch (ClassCastException expected) {}
        try { ConsumerKt.checked(3); throw new AssertionError(); } catch (ClassCastException expected) {}
        try { ConsumerKt.checked(null); throw new AssertionError(); } catch (NullPointerException expected) {}
    }
}

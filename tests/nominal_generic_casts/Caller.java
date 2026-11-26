import fixture.*;
public class Caller {
    public static void main(String[] args) {
        Box<String> ok = new Box<>("one");
        Box<Integer> wrong = new Box<>(1);
        if (ConsumerKt.cast(ok) != ok || ConsumerKt.safe("x") != null || ConsumerKt.safe(null) != null ||
            !ConsumerKt.test(wrong) || ConsumerKt.test(null) || !ConsumerKt.read(ok).equals("one")) throw new AssertionError();
        if ((Object) ConsumerKt.safe(wrong) != wrong) throw new AssertionError();
        try { ConsumerKt.read(wrong); throw new AssertionError(); } catch (ClassCastException expected) { }
        try { ConsumerKt.cast("x"); throw new AssertionError(); } catch (ClassCastException expected) { }
        try { ConsumerKt.cast(null); throw new AssertionError(); } catch (NullPointerException expected) { }
    }
}

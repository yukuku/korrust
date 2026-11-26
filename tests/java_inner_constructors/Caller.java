public class Caller {
    public static void main(String[] args) {
        if (model.ConsumerKt.read() != 9 || model.Outer.reads != 2 || model.ConsumerKt.empty(new model.Outer(1)) != 4) throw new AssertionError();
        try { model.ConsumerKt.nullOuter(); throw new AssertionError(); } catch (NullPointerException expected) {
            if (!"nil(...) must not be null".equals(expected.getMessage()) || model.Outer.reads != 2) throw expected;
        }
    }
}

public class Caller {
    public static void main(String[] args) {
        if (fixtures.ConsumerKt.length("ok") != 2 || fixtures.ConsumerKt.number(4) != 9) throw new AssertionError();
        try { fixtures.ConsumerKt.length(null); throw new AssertionError(); } catch (NullPointerException expected) { }
    }
}

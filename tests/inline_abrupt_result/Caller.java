public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.answer("test") != 4) throw new AssertionError();
    }
}

public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.check() != 2127) throw new AssertionError();
    }
}

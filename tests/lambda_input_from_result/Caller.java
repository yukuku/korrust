public class Caller {
    public static void main(String[] args) {
        if (!"abcd".equals(ConsumerKt.text()) || ConsumerKt.count() != 7) throw new AssertionError();
    }
}

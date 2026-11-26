public class Caller {
    public static void main(String[] args) {
        if (!"v7".equals(ConsumerKt.check())) throw new AssertionError();
    }
}

public class Caller {
    public static void main(String[] args) {
        if (!"ok".equals(ConsumerKt.message()) || !"value".equals(ConsumerKt.qualified())) throw new AssertionError();
    }
}

public class Caller {
    public static void main(String[] args) {
        if (!"ok".equals(ConsumerKt.read())) throw new AssertionError();
    }
}

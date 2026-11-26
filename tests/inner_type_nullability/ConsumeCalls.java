public final class ConsumeCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consume()) throw new AssertionError();
        System.out.println("OK");
    }
}

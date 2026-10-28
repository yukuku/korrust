public final class ConsumerCalls {
    public static void main(String[] args) {
        if (!newinterop.consumer.ConsumerKt.result()) throw new AssertionError("consumer");
        System.out.println("OK");
    }
}

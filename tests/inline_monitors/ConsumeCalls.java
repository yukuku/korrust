public final class ConsumeCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consume(new Object())) throw new AssertionError();
        System.out.println("OK");
    }
}

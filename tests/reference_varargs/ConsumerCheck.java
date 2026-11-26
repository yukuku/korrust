public final class ConsumerCheck {
    public static void main(String[] args) {
        if (ConsumerKt.consume() != 15) throw new AssertionError("consumer");
        System.out.println("OK");
    }
}

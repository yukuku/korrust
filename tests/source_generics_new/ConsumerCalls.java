public final class ConsumerCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consume().equals("OK")) throw new AssertionError();
        System.out.println("OK");
    }
}

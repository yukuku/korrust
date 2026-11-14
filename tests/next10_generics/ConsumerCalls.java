public final class ConsumerCalls {
    public static void main(String[] args) {
        if (!use.ConsumerKt.consume().equals("OKOK17")) throw new AssertionError();
        System.out.println("OK");
    }
}

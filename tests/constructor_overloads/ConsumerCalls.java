public final class ConsumerCalls {
    public static void main(String[] args) {
        String result = constructors.ConsumerKt.consume();
        if (!result.equals("OK")) throw new AssertionError(result);
        System.out.println("OK");
    }
}

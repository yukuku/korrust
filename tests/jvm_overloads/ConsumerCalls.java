public final class ConsumerCalls {
    public static void main(String[] args) {
        String result = overloads.consumer.ConsumerKt.result();
        if (!result.equals("1:K:3")) throw new AssertionError(result);
        System.out.println("OK");
    }
}

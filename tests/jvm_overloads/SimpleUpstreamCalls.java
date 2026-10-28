public final class SimpleUpstreamCalls {
    public static void main(String[] args) {
        String value = SimpleKt.box();
        if (!value.equals("OK")) throw new AssertionError(value);
        System.out.println("OK");
    }
}

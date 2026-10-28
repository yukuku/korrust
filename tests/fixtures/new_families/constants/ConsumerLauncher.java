public final class ConsumerLauncher {
    public static void main(String[] args) {
        String result = newfamilies.constants.consumer.ConsumerKt.check();
        if (!"OK".equals(result)) throw new AssertionError(result);
        System.out.println(result);
    }
}

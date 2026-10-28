public final class IntegrationLauncher {
    public static void main(String[] args) {
        String result = newfamilies.integrationconsumer.IntegrationConsumerKt.check();
        if (!"OK".equals(result)) throw new AssertionError(result);
        System.out.println(result);
    }
}

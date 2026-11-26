import anonymousconsumer.ConsumerKt;
public final class ConsumerCheck {
    public static void main(String[] args) throws Exception {
        Check.suspended(() -> ConsumerKt.consumer(), 1, 13);
        Check.suspended(() -> ConsumerKt.consumerGeneric(), 1, "k9");
        Check.suspended(() -> ConsumerKt.consumerSam(), 1, 12);
        if (!ConsumerKt.staticValue().equals("ok")) throw new AssertionError();
        System.out.println("OK");
    }
}

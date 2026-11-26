public final class ConsumerCheck {
    public static void main(String[] args) throws Exception {
        Check.suspended(receiverconsumer.ConsumerKt::use, 10);
        Check.suspended(receiverconsumer.ConsumerKt::nested, 9);
        System.out.println("OK");
    }
}

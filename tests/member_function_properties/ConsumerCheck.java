public final class ConsumerCheck {
    public static void main(String[] args) {
        if (memberfunctionconsumer.ConsumerKt.run() != 150) throw new AssertionError();
        System.out.println("OK");
    }
}

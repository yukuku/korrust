public class ConsumeCalls {
    public static void main(String[] args) {
        if (!bodyconsumer.ConsumerKt.consume().equals("NOKLMP")) throw new AssertionError();
        System.out.println("OK");
    }
}

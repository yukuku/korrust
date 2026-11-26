public class ConsumerCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consumer().equals("bodytokenown")) throw new AssertionError();
        System.out.println("consumer-ok");
    }
}

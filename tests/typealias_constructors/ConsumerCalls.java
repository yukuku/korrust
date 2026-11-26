public class ConsumerCalls {
    public static void main(String[] args) {
        if (!ConsumerKt.consumer().equals("swapnestedcopy")) throw new AssertionError();
        System.out.println("consumer-ok");
    }
}

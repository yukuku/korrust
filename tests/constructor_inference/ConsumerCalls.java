public class ConsumerCalls {
    public static void main(String[] args) {
        if (!constructors.downstream.ConsumerKt.value().equals("metadata")) throw new AssertionError();
        System.out.println("consumer-ok");
    }
}

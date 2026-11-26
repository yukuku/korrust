import consumers.ConsumerKt;

public class Caller {
    public static void main(String[] args) {
        if (!"first".equals(ConsumerKt.first())) throw new AssertionError();
    }
}

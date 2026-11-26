public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.wide(0) != null || ConsumerKt.wide(1) != Nodes.first
                || ConsumerKt.narrow() != Nodes.first || !ConsumerKt.asArgument()) throw new AssertionError();
    }
}

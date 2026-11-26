public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.both() != 14 || new Holder(new Nodes.Base()).getItem().n() != 2
                || ConsumerKt.take(new Nodes.Base()) != 2) throw new AssertionError();
    }
}

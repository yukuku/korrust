public class Caller {
    public static void main(String[] args) {
        if (ConsumerKt.count() != 2 || ConsumerKt.objects() != 1) throw new AssertionError();
    }
}

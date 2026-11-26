public class Caller {
    public static void main(String[] args) {
        String[] a = {null, "two"};
        if (ConsumerKt.count(a) != 2 || ConsumerKt.text(a) != null) throw new AssertionError();
    }
}

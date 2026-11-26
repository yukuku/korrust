public class Caller {
    public static void main(String[] args) {
        String[] values = {"x", null, "x"};
        if (ConsumerKt.position(values, "x") != 0 || ConsumerKt.position(values, "y") != -1)
            throw new AssertionError();
        if (ConsumerKt.nullable(values) != 1 || ConsumerKt.fromJava() != 0)
            throw new AssertionError();
        if (ConsumerKt.projected(new Number[]{1.0, 2}, 2) != 1) throw new AssertionError();
    }
}

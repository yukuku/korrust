public class Caller {
    public static void main(String[] args) {
        var expected = java.util.List.of(1, 0, 2, 4);
        if (!expected.equals(new sample.Example().getNumbers()) ||
            !expected.equals(sample.ConsumerKt.getGlobal())) throw new AssertionError();
    }
}

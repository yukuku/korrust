public class Caller {
    public static void main(String[] args) {
        if (fixture.ConsumerKt.test() != 2) throw new AssertionError();
    }
}

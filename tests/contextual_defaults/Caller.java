public class Caller {
    public static void main(String[] args) {
        if (!fixtures.ConsumerKt.checkDefaults()) throw new AssertionError();
    }
}

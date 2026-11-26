public class Caller {
    public static void main(String[] args) {
        if (!new use.Host().run().equals("sum7") || !use.Host.Companion.pick().equals("one4")) throw new AssertionError();
    }
}

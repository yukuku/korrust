public class Caller {
    public static void main(String[] args) {
        if (!"changed7".equals(pkg.ConsumerKt.run(new pkg.Child()))) throw new AssertionError();
    }
}

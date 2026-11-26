public class Caller {
    public static void main(String[] args) {
        Loop.count = 0;
        new Worker().drain();
        if (Loop.count != 4) throw new AssertionError();
        Loop.count = 0;
        if (ConsumerKt.runWide(9L) != 10L || Loop.count != 4) throw new AssertionError();
    }
}

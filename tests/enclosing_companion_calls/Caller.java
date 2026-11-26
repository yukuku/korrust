public class Caller {
    public static void main(String[] args) {
        if (new Shell.Middle.Inner().score() != 53) throw new AssertionError();
    }
}

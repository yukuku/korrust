public class Caller {
    public static void main(String[] args) {
        if (new Holder(null).read() != -1 || new Holder(new Token()).read() != 12) throw new AssertionError();
    }
}

public class Caller {
    public static void main(String[] args) {
        caller.Counter c = new caller.Counter(0);
        if (caller.ConsumerKt.length("abc", c) != 3 || c.getHits() != 0) throw new AssertionError();
        if (caller.ConsumerKt.length(null, c) != -1 || c.getHits() != 1) throw new AssertionError();
    }
}

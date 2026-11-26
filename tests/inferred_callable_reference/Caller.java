import transform.Words;
public class Caller {
    public static void main(String[] args) {
        Words w = new Words();
        if (!ConsumerKt.read(w).equals("n4") || ConsumerKt.length(w) != 5 || !ConsumerKt.top().equals("number9")) throw new AssertionError();
    }
}
